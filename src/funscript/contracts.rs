use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize, Clone)]
pub struct Funscript {
    #[serde(rename = "version")]
    pub version: String,

    #[serde(rename = "inverted")]
    pub inverted: bool,

    #[serde(rename = "range")]
    pub range: i64,

    #[serde(rename = "actions")]
    pub actions: Vec<Action>,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn funscript_before_first() {
        let s = Funscript {
            actions: vec![
                Action {
                    at: Duration::from_secs(1),
                    pos: 1,
                },
                Action {
                    at: Duration::from_secs(2),
                    pos: 2,
                },
                Action {
                    at: Duration::from_secs(3),
                    pos: 3,
                },
                Action {
                    at: Duration::from_secs(3),
                    pos: 3,
                },
            ],
            inverted: Default::default(),
            range: Default::default(),
            version: String::new(),
        };

        assert_eq!(
            (None, Some(Duration::from_secs(1))),
            s.get_action_at(Duration::ZERO)
        );
    }

    #[test]
    fn funscript_in_between() {
        let s = Funscript {
            actions: vec![
                Action {
                    at: Duration::from_secs(1),
                    pos: 1,
                },
                Action {
                    at: Duration::from_secs(2),
                    pos: 2,
                },
                Action {
                    at: Duration::from_secs(3),
                    pos: 3,
                },
                Action {
                    at: Duration::from_secs(3),
                    pos: 3,
                },
            ],
            inverted: Default::default(),
            range: Default::default(),
            version: String::new(),
        };

        assert_eq!(
            (Some(2), Some(Duration::from_secs(3))),
            s.get_action_at(Duration::from_millis(2500))
        );
    }

    #[test]
    fn funscript_after_last() {
        let s = Funscript {
            actions: vec![
                Action {
                    at: Duration::from_secs(1),
                    pos: 1,
                },
                Action {
                    at: Duration::from_secs(2),
                    pos: 2,
                },
                Action {
                    at: Duration::from_secs(3),
                    pos: 3,
                },
                Action {
                    at: Duration::from_secs(3),
                    pos: 3,
                },
            ],
            inverted: Default::default(),
            range: Default::default(),
            version: String::new(),
        };

        assert_eq!(
            (Some(3), None),
            s.get_action_at(Duration::from_secs(4))
        );
    }
}

impl Funscript {
    pub fn end(&self) -> Option<Duration> {
        self.actions.last().map(|a| a.at)
    }

    pub fn get_action_at(&self, t: Duration) -> (Option<u8>, Option<Duration>) {
        let index = self.actions.partition_point(|a| a.at <= t);
        (
            index
                .checked_sub(1)
                .and_then(|index| self.actions.get(index).map(|a| a.pos)),
            self.actions.get(index).map(|a| a.at),
        )
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Action {
    #[serde(rename = "pos")]
    pub pos: u8,

    #[serde(rename = "at", deserialize_with = "duration_from_millis")]
    pub at: Duration,
}

fn duration_from_millis<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    u64::deserialize(deserializer).map(|millis| Duration::from_millis(millis))
}
