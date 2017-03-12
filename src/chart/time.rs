use super::duration::*;
use std::cmp::Ordering;

#[derive(Debug, Eq, Copy, Clone)]
pub struct ChartTime {
	week: u32,
	day: Option<u32>,
	quarter: Option<u32>
}

impl Ord for ChartTime {
    fn cmp(&self, other: &ChartTime) -> Ordering {
        self.get_quarter().cmp(&other.get_quarter())
    }
}

impl PartialOrd for ChartTime {
    fn partial_cmp(&self, other: &ChartTime) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ChartTime {
    fn eq(&self, other: &ChartTime) -> bool {
        self.get_quarter() == other.get_quarter()
    }
}

impl ChartTime {

	pub fn new(desc: &str) -> Result<ChartTime, String> {
		let v: Vec<&str> = desc.split('.').collect();
		if v.len() > 3 {

			return Err(format!("Too many parts in time string: {}", desc));
		}		
		if v.len() == 0 {
			return Err(format!("No info in time string: {}", desc));
		}		

		let week: u32;
		let day: Option<u32>;
		let quarter: Option<u32>;
		match v[0].parse::<u32>() {
			Ok(num) => {
				week = num;
			},
			Err(e) => {
				return Err(format!("Failed to parse week from time string:{}, {}", desc, e.to_string()));
			}
		}
		if v.len() > 1 {
			match v[1].parse::<u32>() {
				Ok(num) => {
					if num >= 1 && num <= 5 {
						day = Some(num);
					} else {
						return Err(format!("Failed to parse day from time string:{}, value is out of range", desc));
					}
				},
				Err(e) => {
					return Err(format!("Failed to parse day from time string:{}, {}", desc, e.to_string()));
				}
			}
		} else {
			day = None
		}

		if v.len() > 2 {
			match v[2].parse::<u32>() {
				Ok(num) => {
					if num >= 1 && num <= 4 {
						quarter = Some(num);
					} else {
						return Err(format!("Failed to parse quarter from time string:{}, value is out of range", desc));
					}
				},
				Err(e) => {
					return Err(format!("Failed to parse quarter from time string:{}, {}", desc, e.to_string()));
				}
			}
		} else {
			quarter = None
		}

		Ok(ChartTime { week: week, day: day, quarter: quarter })
	}

	/// Get the first (0-based) quarter associated with this time
	pub fn get_quarter(&self) -> u32 {
		let mut q = (self.week - 1) * 20;
		q += match self.day {
				Some(day) => (day - 1) * 4,
				None => 0,
		};
		q += match self.quarter {
				Some(quarter) => (quarter - 1),
				None => 0,
		};

		q
	}

	/// Get the duration of this time
	///
	/// A time can be specified with different levels of precision.  This
	/// returns the duration from the first possible quarter to the last.
	pub fn get_duration(&self) -> Duration {
		match self.quarter {
			Some(_) => Duration::new_quarters(1),
			None => {
				match self.day {
					Some(_) => Duration::new_quarters(4),
					None => Duration::new_quarters(20)
				}
			},
		}
	}
}