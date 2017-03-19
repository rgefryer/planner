use super::time::*;
use super::duration::*;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct ChartPeriod {
    first: u32,
    last: u32,
}

impl ChartPeriod {
    pub fn new_from_time_dur(time: &ChartTime, dur: &Duration) -> Option<ChartPeriod> {
        if dur.quarters() > 0 {
            Some(ChartPeriod {
                     first: time.get_quarter(),
                     last: time.get_quarter() + dur.quarters() as u32 - 1,
                 })
        } else {
            None
        }
    }

    pub fn new(first: u32, last: u32) -> Option<ChartPeriod> {
        if first <= last {
            Some(ChartPeriod {
                     first: first,
                     last: last,
                 })
        } else {
            None
        }
    }

    pub fn intersect(&self, other: &ChartPeriod) -> Option<ChartPeriod> {
        if self.first >= other.first && self.first <= other.last {
            if self.last > other.last {
                Some(ChartPeriod {
                         first: self.first,
                         last: other.last,
                     })
            } else {
                Some(ChartPeriod {
                         first: self.first,
                         last: self.last,
                     })
            }

        } else if other.first >= self.first && other.first <= self.last {
            if other.last > self.last {
                Some(ChartPeriod {
                         first: other.first,
                         last: self.last,
                     })
            } else {
                Some(ChartPeriod {
                         first: other.first,
                         last: other.last,
                     })
            }
        } else {
            None
        }
    }

    pub fn union(&self, other: &ChartPeriod) -> Option<ChartPeriod> {
        if self.first >= other.first && self.first <= other.last {
            if self.last > other.last {
                Some(ChartPeriod {
                         first: other.first,
                         last: self.last,
                     })
            } else {
                Some(ChartPeriod {
                         first: other.first,
                         last: other.last,
                     })
            }

        } else if other.first >= self.first && other.first <= self.last {
            if other.last > self.last {
                Some(ChartPeriod {
                         first: self.first,
                         last: other.last,
                     })
            } else {
                Some(ChartPeriod {
                         first: self.first,
                         last: self.last,
                     })
            }
        } else {
            None
        }

    }

    pub fn get_first(&self) -> u32 {
        self.first
    }

    pub fn get_last(&self) -> u32 {
        self.last
    }
}
