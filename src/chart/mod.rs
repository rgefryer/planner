#![allow(dead_code)]
mod duration;
mod timerow;
mod readfile;
mod nodes;
mod file;
mod time;

#[cfg(test)]
mod tests;

pub use self::readfile::read_config;
pub use self::nodes::ConfigNode;
pub use self::duration::*;
pub use self::time::*;
pub use self::timerow::*;

use std::collections::HashMap;

/// Strategy for scheduling child nodes
#[derive(Debug, Eq, PartialEq)]
pub enum SchedulingStrategy {

	/// The child nodes must be completed in order; no
	/// work on child 2 until child 1 is complete.
	Serial,

	/// The children can be worked on at the same time.
	/// However, resources are allocated for the children
	/// in the order they are defined.
	Parallel
}

/// Strategy for allocating the budget
#[derive(Debug, Eq, PartialEq)]
pub enum ResourcingStrategy {

	/// Allocated on a weekly rate, calculated quarterly.
	/// 4 quarters management for every 20 quarters managees
	/// (when the manager is present).  Calculated after 
	/// non-managed tasks have been removed.
	Management,

	/// Take the plan value, pro-rata it across the remaining 
	/// time, subtract any future commitments, then smear the
	/// remainder.
	///
	/// Warn if this means that the allocated resource does
	/// not match the plan.
	///
	/// This is typically used for overheads, which anticipate
	/// a steady cost over the entire period.
	SmearProRata,

	/// Take the plan value, subtract commitments, and smear
	/// the remainder across the remaining time.  The smearing ignores
	/// existing commitments - ie the remaining costs are smeared 
	/// across the quarters that are currently empty.
	///
	/// This is typically used for fixed costs, where failure
	/// to use them early in the plan means more costs later.
	SmearRemaining,

	/// Allocate all of the plan asap.
	///
	/// This is typically used for task work.  It can only
	/// be scheduled after the smeared resources.
	FrontLoad,

	/// Like FrontLoad, but allocated from the end of the period.
	BackLoad,

	/// ProdSFR is a special-case of SmearRemaining, where 20% of the
	/// remaining costs are smeared, and the other 80% are back-
	/// filled at the end of the period.
	ProdSFR,

}

/// A row of the Gantt Chart, representing an item of work
#[derive(Debug)]
struct TaskRow {

	/// Scheduled (or completed) times
	times: ChartTimeRow,

	/// Time currently agreed for this activity
	budget: Duration,

	/// How the time should be allocated
	time_allocation: ResourcingStrategy,

	/// Current slip/gain, within plan
	gain: Duration,

	/// Original budget for this work.  Not used in calculations.
	original_budget: Duration,

	/// Historical slip/gain.  This is already reflected in the budget,
	/// and is not used in calculations.
	historical_gain: Duration,
}


/// An entire Gantt chart - a collection of rows
#[derive(Debug)]
struct GanttChart {

	/// Rows representing task work
	work_rows: Vec<TaskRow>,

	/// Rows representing people - resource to be allocated
	resource_rows: HashMap<String, ChartTimeRow>,

	/// Length of the chart
	chart_length: Duration,

}