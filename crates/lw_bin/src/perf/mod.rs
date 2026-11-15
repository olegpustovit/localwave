//! Startup selection for the isolated signed performance matrix; no argument work runs in frames.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Workload {
    Songs,
    SongIndicators,
    Artists,
    Drop,
    Files,
    Settings,
    Waveform,
    Artwork,
    Find,
    QueueSheet,
    QueueDrag,
    Queue,
}

impl Workload {
    pub fn from_args(args: &[String]) -> Result<Option<Self>, String> {
        let mut enabled = false;
        let mut selected = None;
        for arg in args {
            let workload = match arg.as_str() {
                "--performance-check" => {
                    enabled = true;
                    continue;
                }
                "--performance-song-indicators" => Self::SongIndicators,
                "--performance-artists" => Self::Artists,
                "--performance-drop" => Self::Drop,
                "--performance-files" => Self::Files,
                "--performance-settings" => Self::Settings,
                "--performance-waveform" => Self::Waveform,
                "--performance-artwork" => Self::Artwork,
                "--performance-find" => Self::Find,
                "--performance-queue-sheet" => Self::QueueSheet,
                "--performance-queue-drag" => Self::QueueDrag,
                "--performance-queue" => Self::Queue,
                _ if arg.starts_with("--performance-") => {
                    return Err(format!("Unknown performance option: {arg}"));
                }
                _ => continue,
            };
            if let Some((previous, previous_arg)) = selected
                && previous != workload
            {
                return Err(format!(
                    "Performance workloads must run separately: {previous_arg} and {arg}"
                ));
            }
            selected = Some((workload, arg));
        }
        match (enabled, selected) {
            (true, selected) => Ok(Some(selected.map_or(Self::Songs, |(workload, _)| workload))),
            (false, Some((_, arg))) => Err(format!("{arg} requires --performance-check")),
            (false, None) => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workload_args_select_one_complete_matrix_case_and_reject_mixed_fixtures() {
        let args = |values: &[&str]| {
            values
                .iter()
                .map(|value| (*value).to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            Workload::from_args(&args(&["localwave", "--smoke-test"])),
            Ok(None)
        );
        assert_eq!(
            Workload::from_args(&args(&["--performance-check"])),
            Ok(Some(Workload::Songs))
        );
        let modes = [
            ("--performance-song-indicators", Workload::SongIndicators),
            ("--performance-artists", Workload::Artists),
            ("--performance-drop", Workload::Drop),
            ("--performance-files", Workload::Files),
            ("--performance-settings", Workload::Settings),
            ("--performance-waveform", Workload::Waveform),
            ("--performance-artwork", Workload::Artwork),
            ("--performance-find", Workload::Find),
            ("--performance-queue-sheet", Workload::QueueSheet),
            ("--performance-queue-drag", Workload::QueueDrag),
            ("--performance-queue", Workload::Queue),
        ];
        for (flag, expected) in modes {
            for values in [
                vec!["--performance-check", flag],
                vec![flag, "--trace-frames", "--performance-check", flag],
            ] {
                assert_eq!(Workload::from_args(&args(&values)), Ok(Some(expected)));
            }
            assert!(Workload::from_args(&args(&[flag])).is_err());
            for (other, _) in modes {
                if flag != other {
                    assert!(
                        Workload::from_args(&args(&["--performance-check", flag, other])).is_err()
                    );
                }
            }
        }
        assert!(
            Workload::from_args(&args(&["--performance-check", "--performance-typo"])).is_err()
        );
        assert!(Workload::from_args(&args(&["--performance-typo"])).is_err());
    }
}
