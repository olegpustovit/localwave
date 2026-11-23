//! Opt-in, cumulative counters. Reporting piggybacks on callbacks to leave an idle app asleep.

pub(super) struct Report {
    next: f64,
    builds: [u64; 6],
}

impl Report {
    pub(super) fn from_env(now: f64) -> Option<Self> {
        (std::env::var_os("PXUI_FRAME_REASONS").is_some_and(|value| value == "1")).then_some(Self {
            next: now + 1.0,
            builds: [0; 6],
        })
    }

    pub(super) fn build(
        &mut self,
        window: bool,
        input: bool,
        player: bool,
        worker: bool,
        perf: bool,
    ) {
        let reason = [window, input, player, worker, perf]
            .iter()
            .position(|&active| active)
            .unwrap_or(5);
        self.builds[reason] += 1;
    }

    pub(super) fn print(&mut self, now: f64, draws: usize, ui: &pxui::Ui) {
        if now < self.next {
            return;
        }
        self.next = now + 1.0;
        let r = ui.frame_reasons();
        let [window, input, player, worker, perf, deadline] = self.builds;
        let curve = r.transition_curve;
        eprintln!(
            "PXUI_FRAME_REASONS t={now:.3} draws={draws} window={window} input={input} player={player} \
             worker={worker} perf={perf} deadline={deadline} requested={} interaction={} transition={} \
             finite_keys={} active_loop={} idle_loop={} stepped={} step_state={} step_build={} \
             step_interaction={} step_deadline={} node_key={} curve_kind={} duration={} points={:?} \
             omega={} count={} positions={:?} values={:?}",
            r.requested,
            r.interaction,
            r.transition,
            r.finite_keys,
            r.active_loop,
            r.idle_loop,
            r.stepped,
            r.step_state,
            r.step_build,
            r.step_interaction,
            r.step_deadline,
            r.transition_key,
            curve.kind,
            curve.duration,
            curve.points,
            curve.omega,
            curve.count,
            curve.positions,
            curve.values,
        );
    }
}
