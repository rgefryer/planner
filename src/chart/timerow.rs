use std;
use std::fmt;
use super::time::*;


/// The time cells for a single Gantt row, split into 1/4 day chunks.
#[derive(Debug)]
pub struct ChartTimeRow {

	/// Cells, as a bit field
	cells: Vec<u8>
}

impl fmt::Display for ChartTimeRow {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

    	let mut output = String::new();
		for cell in &self.cells {
			let mut cell_copy = *cell;
			for _ in 0..8 {
				if cell_copy & 0x01 == 0x01 {
					output = output + "o";
				}
				else {
					output = output + "_";
				}
				cell_copy >>= 1;
			}
		}

        write!(f, "[{}]", output)
    }
}

impl ChartTimeRow {

	/// Create new row with all cells unallocated
	pub fn new() -> ChartTimeRow {
		ChartTimeRow { cells: Vec::new() }
	}

	/// Create new row populated according to the
	/// specified range.
	///
	/// The range takes the form <start>..[<end>], where
	/// start and end are chart times.
	pub fn new_populate_range(range: &str, weeks: u32) -> Result<ChartTimeRow, String> {

		let v: Vec<&str> = range.split("..").collect();
		if v.len() > 2 {
			return Err(format!("Too many parts in range {}", range));
		}
		if v.len() == 0 {
			return Err(format!("Not enough parts in range {}", range));
		}
		let start = try!(ChartTime::new(v[0]).map_err(|e| format!("Invalid range start {}, {}", v[0], e)));
		let end = if v.len() == 1 || v[1].len() == 0 {
			try!(ChartTime::new(&format!("{}", weeks)))
		} else {
			try!(ChartTime::new(v[1]).map_err(|e| format!("Invalid range end {}, {}", v[1], e)))
		};

		if try!(ChartTime::new(&format!("{}", weeks))) < end {
			return Err(format!("Range in {} exceeds chart length of {} weeks", range, weeks));
		}

		let mut ctr = ChartTimeRow::new();
		ctr.set_range(start.get_quarter() .. (end.get_quarter() + end.get_duration().quarters() as u32));

		Ok(ctr)
	}

	/// Set a specific cell
	pub fn set(&mut self, cell: u32) {
		let byte = (cell / 8) as usize;
		let bit = cell % 8;
		let test = 0x01 << bit;

		while self.cells.len() <= byte {
			self.cells.push(0);
		}

		self.cells[byte] |= test;
	}

	/// Unset a specific cell
	pub fn unset(&mut self, cell: u32) {
		let byte = (cell / 8) as usize;
		let bit = cell % 8;
		let test = 0x01 << bit;

		if self.cells.len() > byte {
			self.cells[byte] &= !test;
		}

	}

	/// Test whether a specific cell is set
	pub fn is_set(&self, cell: u32) -> bool {
		let byte = (cell / 8) as usize;
		let bit = cell % 8;
		let test = 0x01 << bit;

		if self.cells.len() < byte + 1 {
			return false;
		}

		self.cells[byte] & test == test
	}

	/// Set a range of cells
	pub fn set_range<'a, I>(&mut self, range: I) 
	  where I: Iterator<Item=u32> {

		for cell in range {
			self.set(cell);
		}
	}

	/// Count the number of cells that are set
	pub fn count(&self) -> u32 {
		let mut count = 0u32;
		for cell in &self.cells {
			let mut cell_copy = *cell;
			while cell_copy != 0 {
				if cell_copy & 0x01 == 0x01 {
					count += 1;
				}
				cell_copy >>= 1;
			}
		}
		count
	}

	/// Transfer a number of cells to another row.  The cells are inserted
	/// from the start of the range, as allowed by existing commitments.
	/// Returns a tuple of
	/// - the last cell transferred (Option)
	/// - the number of cells transferred
	/// - the number of cells that could not be transferred
	pub fn fill_transfer_to<'a, I>(&mut self,
							       dest: &mut ChartTimeRow, 
							       count: u32, 
							       range: I) -> (Option<u32>, u32, u32)
	  where I: Iterator<Item=u32> {

	  	let mut to_allocate = count;
	  	let mut last_transfer: Option<u32> = None;

		for cell in range {
	  		if self.is_set(cell) && !dest.is_set(cell) {
	  			to_allocate -= 1;
	  			self.unset(cell);
	  			dest.set(cell);
	  			last_transfer = Some(cell);

	  			if to_allocate == 0 {
	  				break;
	  			}
	  		}
		}
	  	
	  	(last_transfer, count - to_allocate, to_allocate)
	}

	/// Transfer a number of cells to another row.  The cells are inserted
	/// from the end of the range, as allowed by existing commitments.
	/// If not all cells can be transferred, returns an error with the number 
	/// of unallocated cells.  If successful, returns the last cell to be
	/// transferred.
	pub fn reverse_fill_transfer_to<'a, I>(&mut self,
							   dest: &mut ChartTimeRow, 
							   count: u32, 
							   range: I) -> Result<u32, u32>
	  where I: std::iter::DoubleEndedIterator<Item=u32> {

	  	let mut to_allocate = count;

		for cell in range.rev() {
	  		if self.is_set(cell) && !dest.is_set(cell) {
	  			to_allocate -= 1;
	  			self.unset(cell);
	  			dest.set(cell);

	  			if to_allocate == 0 {
	  				return Ok(cell);
	  			}
	  		}
		}
	  	
  		Err(to_allocate)
	}

	/// Transfer a number of cells to another row.  The cells are smoothed
	/// out over the range, as much as is allowed by existing commitments.
	/// If not all cells can be transferred, returns an error with the number 
	/// of unallocated cells.
	pub fn smear_transfer_to<'a, I>(&mut self,
								dest: &mut ChartTimeRow, 
								count: u32, 
								range: I) -> Result<(), u32>
	  where I: Iterator<Item=u32> {

	  	let candidate_cells = range.collect::<Vec<u32>>();
	  	let mut want_allocated = 0f64;
	  	let mut allocated = 0u32;
	  	let mut transferred_this_run = 1u32;

	  	'outer_loop: while transferred_this_run != 0 && allocated != count {

		  	let amount_per_cell = (count - allocated) as f64 / candidate_cells.len() as f64;
		  	transferred_this_run = 0;
		  	for cell in &candidate_cells {
		  		want_allocated += amount_per_cell;
		  		if want_allocated > (allocated as f64) && self.is_set(*cell) && !dest.is_set(*cell) {
		  			allocated += 1;
		  			transferred_this_run += 1;
		  			self.unset(*cell);
		  			dest.set(*cell);

		  			if allocated == count {
		  				break 'outer_loop;
		  			}
		  		}
		  	}
	  	}

	  	if allocated == count {
	  		Ok(())
	  	}
	  	else {
	  		Err(count - allocated)
	  	}
	}
}


