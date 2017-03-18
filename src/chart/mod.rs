#![allow(dead_code)]

use std::cell::RefCell;
use std::rc::Rc;

mod duration;
mod timerow;
mod readfile;
mod nodes;
mod file;
mod time;
mod web;

#[cfg(test)]
mod tests;

pub use self::readfile::read_config;
pub use self::nodes::ConfigNode;
pub use self::duration::*;
pub use self::time::*;
pub use self::timerow::*;

#[cfg(not(test))]
pub use self::web::serve_web;

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

pub fn generate_chart_nodes() -> Result<Rc<RefCell<ConfigNode>>, String> {

	// Read in the config file
    let mut f = try!(read_config(&("config.txt".to_string())));

    // Generate the config nodes
    let rc_root = Rc::new(RefCell::new(ConfigNode::new("root", 0, 0, 0)));

    // Isolate borrowing root, so that we can return rc_root
    {
    	let mut root = rc_root.borrow_mut();
    	try!(root.consume_config(Some(&rc_root), &mut f));
    	println!("Read {} nodes", root.count_nodes());

    	// Set up the resource information
    	try!(root.fill_in_gantt());

    	// Display the gantt chart
    	//try!(root.display_gantt());
    }

	Ok(rc_root)
}
