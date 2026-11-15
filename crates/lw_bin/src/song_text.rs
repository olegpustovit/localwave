//! Music-specific viewport selection over pxui's owned text preparation API. One persistent worker,
//! one coalesced request and 64 deliveries; superseded requests and adopted/rejected storage retire
//! on the worker outside its mutex. No playback/DB work shares this queue. UI adoption and request
//! delivery belong inside the measured build. Late preparation never delays input or hides rows.
use lw_ui::{
    catalog::Catalog,
    song_text::{ROW_LAYOUTS, Window},
};
use pxui::{PreparedText, RetiredText, Text, Ui};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

const CAPACITY: usize = 64;
struct Request {
    catalog: Catalog,
    window: Window,
    epoch: u64,
}
struct Ready {
    text: PreparedText,
    epoch: u64,
}
type Returned = Result<RetiredText, PreparedText>;
type Retirements = [Option<Returned>; CAPACITY];
struct Mailbox {
    #[cfg(test)]
    rows: usize,
    request: Option<Request>,
    retired_request: Option<Request>,
    shaper: Option<Text>,
    ready: VecDeque<Ready>,
    retired: Retirements,
    cancelled: bool,
    latest: u64,
    closed: bool,
}
struct Shared {
    mailbox: Mutex<Mailbox>,
    changed: Condvar,
    wake: extern "C" fn(),
}

pub struct SongTextWorker {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
    sent: Option<Window>,
    epoch: u64,
    refresh: bool,
}
impl SongTextWorker {
    /// Start before raising the UI's QoS; only the fork/reservation touches the UI.
    pub fn open(ui: &mut Ui, wake: extern "C" fn()) -> std::io::Result<Self> {
        let text = ui
            .text_service()
            .fork()
            .ok_or_else(|| std::io::Error::other("Unable to fork song text fonts"))?;
        let shared = Arc::new(Shared {
            mailbox: Mutex::new(Mailbox {
                #[cfg(test)]
                rows: 0,
                request: None,
                retired_request: None,
                shaper: None,
                ready: VecDeque::with_capacity(CAPACITY),
                retired: [const { None }; CAPACITY],
                cancelled: false,
                latest: 0,
                closed: false,
            }),
            changed: Condvar::new(),
            wake,
        });
        let owner = shared.clone();
        let thread = std::thread::Builder::new()
            .name("localwave-song-text".into())
            .spawn(move || run(owner, text))?;
        Ok(Self {
            shared,
            thread: Some(thread),
            sent: None,
            epoch: 0,
            refresh: false,
        })
    }
    fn advance(&mut self) {
        self.epoch = self
            .epoch
            .checked_add(1)
            .expect("Song text identity exhausted");
    }
    pub fn ready(&self) -> bool {
        self.refresh || !self.shared.mailbox.lock().unwrap().ready.is_empty()
    }
    /// Between frames. Return every replacement/refusal to the worker, retaining backpressure.
    pub fn poll(&mut self, ui: &mut Ui) -> Result<usize, String> {
        if self.refresh && self.shared.mailbox.lock().unwrap().shaper.is_none() {
            let text = ui
                .text_service()
                .fork()
                .ok_or("Unable to refresh song text fonts")?;
            self.advance();
            self.sent = None;
            self.refresh = false;
            let mut inbox = self.shared.mailbox.lock().unwrap();
            inbox.latest = self.epoch;
            inbox.shaper = Some(text);
            inbox.cancelled = true;
            drop(inbox);
            self.shared.changed.notify_one();
        }
        let mut batch = [const { None }; CAPACITY];
        {
            let mut inbox = self.shared.mailbox.lock().unwrap();
            if inbox.retired.iter().any(Option::is_some) {
                return Ok(0);
            }
            for slot in &mut batch {
                *slot = inbox.ready.pop_front();
            }
        }
        let mut retired = [const { None }; CAPACITY];
        let mut installed = 0;
        for (slot, ready) in retired.iter_mut().zip(batch) {
            let Some(ready) = ready else {
                continue;
            };
            *slot = Some(if ready.epoch != self.epoch {
                Err(ready.text)
            } else {
                match ui.text_service().install(ready.text) {
                    Ok(storage) => {
                        installed += 1;
                        Ok(storage)
                    }
                    Err((_, text)) => {
                        self.refresh = true;
                        Err(text)
                    }
                }
            });
        }
        if retired.iter().any(Option::is_some) {
            self.shared.mailbox.lock().unwrap().retired = retired;
            self.shared.changed.notify_one();
        }
        Ok(installed)
    }
    /// Coalesce a bounded viewport. A full retirement slot defers the new request unchanged.
    pub fn request(&mut self, catalog: &Catalog, window: Option<Window>) -> bool {
        if window.is_some_and(|window| !window.matches(catalog, window.route())) {
            return false;
        }
        if window == self.sent {
            return true;
        }
        if window.is_none() {
            self.advance();
            self.sent = None;
            let mut inbox = self.shared.mailbox.lock().unwrap();
            inbox.latest = self.epoch;
            inbox.cancelled = true;
            drop(inbox);
            self.shared.changed.notify_one();
            return true;
        }
        let mut inbox = self.shared.mailbox.lock().unwrap();
        if inbox.request.is_some() && inbox.retired_request.is_some() {
            return false;
        }
        let window = window.unwrap();
        if !self.sent.is_some_and(|sent| sent.same_layout(&window)) {
            self.epoch = self
                .epoch
                .checked_add(1)
                .expect("Song text identity exhausted");
        }
        if let Some(previous) = inbox.request.take() {
            inbox.retired_request = Some(previous);
        }
        inbox.request = Some(Request {
            catalog: catalog.clone(),
            window,
            epoch: self.epoch,
        });
        inbox.latest = self.epoch;
        self.sent = Some(window);
        drop(inbox);
        self.shared.changed.notify_one();
        true
    }
}
impl Drop for SongTextWorker {
    fn drop(&mut self) {
        self.shared.mailbox.lock().unwrap().closed = true;
        self.shared.changed.notify_one();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run(shared: Arc<Shared>, mut text: Text) {
    let mut request: Option<Request> = None;
    let mut seen = HashSet::with_capacity(lw_ui::song_text::MAX_ROWS);
    let mut delivered = HashMap::with_capacity(lw_ui::song_text::MAX_ROWS * ROW_LAYOUTS);
    let mut next = 0;
    loop {
        let (incoming, retired, retired_request, shaper, cancelled, latest) = {
            let mut inbox = shared.mailbox.lock().unwrap();
            loop {
                let work = request
                    .as_ref()
                    .is_some_and(|request| next < request.window.rows().end)
                    && inbox.ready.len() + ROW_LAYOUTS <= CAPACITY;
                if inbox.closed
                    || inbox.request.is_some()
                    || inbox.retired_request.is_some()
                    || inbox.shaper.is_some()
                    || inbox.cancelled
                    || inbox.retired.iter().any(Option::is_some)
                    || work
                {
                    break;
                }
                inbox = shared.changed.wait(inbox).unwrap();
            }
            if inbox.closed {
                let remaining = (
                    inbox.request.take(),
                    inbox.retired_request.take(),
                    inbox.shaper.take(),
                    std::mem::take(&mut inbox.ready),
                    std::mem::replace(&mut inbox.retired, [const { None }; CAPACITY]),
                );
                drop(inbox);
                drop(remaining);
                return;
            }
            (
                inbox.request.take(),
                std::mem::replace(&mut inbox.retired, [const { None }; CAPACITY]),
                inbox.retired_request.take(),
                inbox.shaper.take(),
                std::mem::take(&mut inbox.cancelled),
                inbox.latest,
            )
        };
        drop(retired);
        drop(retired_request);
        if let Some(shaper) = shaper {
            text = shaper;
            request = None;
            seen.clear();
            delivered.clear();
        }
        if cancelled {
            request = None;
            seen.clear();
            delivered.clear();
        }
        if let Some(incoming) = incoming
            && incoming.epoch == latest
        {
            if !request
                .as_ref()
                .is_some_and(|old| old.epoch == incoming.epoch)
            {
                seen.clear();
                delivered.clear();
            }
            delivered.retain(|_, index| {
                *index >= incoming.window.rows().start && *index < incoming.window.rows().end
            });
            seen.retain(|index| {
                *index >= incoming.window.rows().start && *index < incoming.window.rows().end
            });
            next = incoming.window.rows().start;
            request = Some(incoming);
        }
        let Some(current) = &request else {
            continue;
        };
        while next < current.window.rows().end && seen.contains(&next) {
            next += 1;
        }
        if next == current.window.rows().end {
            continue;
        }
        {
            let inbox = shared.mailbox.lock().unwrap();
            if inbox.latest != current.epoch || inbox.ready.len() + ROW_LAYOUTS > CAPACITY {
                continue;
            }
        }
        let row = next;
        next += 1;
        seen.insert(row);
        let mut prepared = match current.window.prepare_row(&current.catalog, row, &mut text) {
            Ok(prepared) => prepared,
            Err(code) => {
                // Oversized/cold layouts keep the ordinary visible rendering path; retry on a new context.
                eprintln!("LocalWave song text row {row}: preparation failed ({code})");
                continue;
            }
        };
        for slot in &mut prepared {
            if let Some((key, _)) = slot.as_ref()
                && delivered.insert(*key, row).is_some()
            {
                drop(slot.take());
            }
        }
        let mut inbox = shared.mailbox.lock().unwrap();
        if inbox.closed || inbox.latest != current.epoch {
            drop(inbox);
            drop(prepared);
            continue;
        }
        for (_, text) in prepared.into_iter().flatten() {
            inbox.ready.push_back(Ready {
                text,
                epoch: current.epoch,
            });
        }
        #[cfg(test)]
        {
            inbox.rows += 1;
        }
        drop(inbox);
        (shared.wake)();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pxui::{FrameIn, Platform};
    use std::time::{Duration, Instant};

    extern "C" fn wake() {}
    fn catalog() -> Catalog {
        let mut catalog = Catalog::new(
            (0..500)
                .map(|id| lw_core::Song {
                    id: Some(id),
                    title: format!("Track {id:05}"),
                    artist: "Example artist".into(),
                    album: "Example album".into(),
                    duration: Some(240.0),
                    ..Default::default()
                })
                .collect(),
        );
        catalog.prepare_tables().unwrap();
        catalog
    }
    fn draw(ui: &mut Ui, catalog: &Catalog, state: &lw_ui::ShellState, width: f32) -> Window {
        ui.frame(
            &FrameIn {
                width,
                height: 872.0,
                reduced_motion: 1,
                ..Default::default()
            },
            &mut |ui| {
                lw_ui::shell(
                    ui,
                    &lw_ui::PlayerView::default(),
                    0.0,
                    catalog,
                    state,
                    &mut None,
                )
            },
        )
        .unwrap();
        state.song_text_window(catalog).unwrap()
    }
    fn wait_full(worker: &SongTextWorker) {
        let until = Instant::now() + Duration::from_secs(1);
        loop {
            let inbox = worker.shared.mailbox.lock().unwrap();
            assert!(inbox.ready.len() <= CAPACITY);
            if inbox.ready.len() + ROW_LAYOUTS > CAPACITY {
                break;
            }
            drop(inbox);
            assert!(
                Instant::now() < until,
                "text worker did not fill its bounded delivery queue"
            );
            std::thread::yield_now();
        }
    }

    #[test]
    fn song_text_worker_fills_before_poll_adopts_without_allocations_and_retires_catalog_on_cancel()
    {
        mem_sys::set_peer(pxui::library_counter);
        let catalog = catalog();
        let state = lw_ui::ShellState::default();
        let mut ui = Ui::create(Platform::Apple, Some(15), &[lw_ui::FONT, lw_ui::MONO]);
        let window = draw(&mut ui, &catalog, &state, 1440.0);
        assert!(
            window.rows().len() < catalog.len()
                && window.rows().len() <= lw_ui::song_text::MAX_ROWS
        );
        let mut worker = SongTextWorker::open(&mut ui, wake).unwrap();
        assert!(worker.request(&catalog, Some(window)));
        wait_full(&worker);
        assert!(
            worker.shared.mailbox.lock().unwrap().rows > 1,
            "preparation must not wait for a frame per row"
        );
        let layouts = ui.text_layout_count();
        let until = Instant::now() + Duration::from_secs(1);
        let mut adopted = 0;
        loop {
            mem_sys::local_reset();
            adopted += worker.poll(&mut ui).unwrap();
            assert_eq!(
                mem_sys::local_alloc_count(),
                0,
                "delivery and retirement must allocate on neither UI-side allocator"
            );
            assert_eq!(ui.text_layout_count(), layouts);
            let inbox = worker.shared.mailbox.lock().unwrap();
            if inbox.rows == window.rows().len()
                && inbox.ready.is_empty()
                && inbox.retired.iter().all(Option::is_none)
            {
                break;
            }
            drop(inbox);
            assert!(
                Instant::now() < until,
                "bounded text window failed to drain"
            );
            std::thread::yield_now();
        }
        assert_eq!(
            adopted,
            window.rows().len() * 2 + 3,
            "only occurrence numbers/titles differ; shared artist/album/time must arrive once"
        );
        ui.frame(
            &FrameIn {
                width: 390.0,
                height: 872.0,
                reduced_motion: 1,
                ..Default::default()
            },
            &mut |ui| {
                lw_ui::shell(
                    ui,
                    &lw_ui::PlayerView::default(),
                    0.0,
                    &catalog,
                    &state,
                    &mut None,
                )
            },
        )
        .unwrap();
        assert!(
            state.song_text_window(&catalog).is_none(),
            "compact views must cancel the previous desktop text window"
        );
        let old = Arc::downgrade(&catalog.library);
        let empty = Catalog::default();
        drop(catalog);
        assert!(
            old.strong_count() > 0,
            "the worker retains the selected source snapshot"
        );
        assert!(worker.request(&empty, None));
        while old.strong_count() != 0 {
            assert!(
                Instant::now() < until,
                "cancelled source must retire on its worker"
            );
            std::thread::yield_now();
        }
        drop(worker);
    }

    #[test]
    fn song_text_worker_retries_stale_fonts_and_coalesces_resized_viewports() {
        let catalog = catalog();
        let state = lw_ui::ShellState::default();
        let mut ui = Ui::create(Platform::Apple, Some(15), &[lw_ui::FONT, lw_ui::MONO]);
        let window = draw(&mut ui, &catalog, &state, 1440.0);
        let mut worker = SongTextWorker::open(&mut ui, wake).unwrap();
        worker.request(&catalog, Some(window));
        wait_full(&worker);
        assert!(ui.text_service().add_font(lw_ui::FONT).is_some());
        assert_eq!(worker.poll(&mut ui).unwrap(), 0);
        assert!(worker.refresh, "a stale generation must trigger a new fork");
        worker.poll(&mut ui).unwrap();
        assert!(!worker.refresh);
        for width in [1420.0, 1400.0, 1380.0] {
            let window = draw(&mut ui, &catalog, &state, width);
            mem_sys::local_reset();
            worker.request(&catalog, Some(window));
            assert_eq!(
                mem_sys::local_alloc_count(),
                0,
                "coalescing a request only clones existing snapshot handles"
            );
        }
        let latest = state.song_text_window(&catalog).unwrap();
        let until = Instant::now() + Duration::from_secs(1);
        let mut adopted = 0;
        loop {
            worker.request(&catalog, Some(latest));
            adopted += worker.poll(&mut ui).unwrap();
            if adopted > 0 && worker.sent == Some(latest) {
                break;
            }
            assert!(
                Instant::now() < until,
                "latest resized viewport must survive coalescing"
            );
            std::thread::yield_now();
        }
        assert!(!worker.refresh);
        drop(worker);
    }
}
