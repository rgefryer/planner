/// Time period
///
/// Can be accessed equally as days (f32) or
/// quarter-days (i32).
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct Duration {
	quarters: i32
}

impl Duration {

	/// Create a Duration from a number of days
	pub fn new_days(days: f32) -> Duration {
		Duration { quarters: (days * 4.0).ceil() as i32 }
	}

	/// Create a Duration from a number of quarters
	pub fn new_quarters(quarters: i32) -> Duration {
		Duration { quarters: quarters}
	}

	/// Create a Duration from a string
	///
	/// Supported formats are:
	/// - "2.25": 2.25 days, or 9 quarters
	/// - "4.5pcy": 4.5 days per calendar year
	/// - "2pcm": 2 days per calendar month
	pub fn new_from_string(amount: &str, over_num_days: &Duration) -> Result<Duration, String> {

		let mut per_year = false;
		let mut per_month = false;
		let mut slice = amount;
		if amount.ends_with("pcy") {
			slice = &amount[.. amount.len() - 3];
			per_year = true;
		}
		if amount.ends_with("pcm") {
			slice = &amount[.. amount.len() - 3];
			per_month = true;
		}

		match slice.parse::<f32>() {
			Ok(number) => {
				if per_year {
					// Calculate number, round to the nearest 0.25 days.
					Ok(Duration::new_quarters((4f32 * number * over_num_days.days() / (5f32 * 52f32)).ceil() as i32))
				} else if per_month {
					// Calculate number, round to the nearest 0.25 days.
					Ok(Duration::new_quarters((4f32 * number * over_num_days.days() / (5f32 * 52f32 / 12f32)).ceil() as i32))
				} else {
					Ok(Duration::new_days(number))
				}
			},
			Err(e) => {
				Err(e.to_string())
			}
		}
	}

	/// The number of days in this Duration
	pub fn days(&self) -> f32 {
		(self.quarters as f32) / 4.0
	}

	/// The number of quarters in this Duration
	pub fn quarters(&self) -> i32 {
		self.quarters
	}

	/// Add a number of days to the Duration
	pub fn add_days(&mut self, days: f32) {
		self.quarters += (days * 4.0) as i32;
	}

	/// Remove a number of days from the Duration
	pub fn remove_days(&mut self, days: f32) {
		self.quarters -= (days * 4.0) as i32;
	}

	/// Add a number of quarters to the Duration
	pub fn add_quarters(&mut self, quarters: i32) {
		self.quarters += quarters;
	}

	/// Remove a number of quarters from the Duration
	pub fn remove_quarters(&mut self, quarters: i32) {
		self.quarters -= quarters;
	}
}

