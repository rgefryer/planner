mod chart;

use std::cell::RefCell;
use std::rc::Rc;

#[cfg(not(test))]
fn do_work() -> Result<(), String> {

	let mut f = try!(chart::read_config(&("config.txt".to_string())));

	let rc_root = Rc::new(RefCell::new(chart::ConfigNode::new("root", 0, 0, 0)));

	// And read the config file
	let mut root = rc_root.borrow_mut();
	try!(root.consume_config(Some(&rc_root), &mut f));
	println!("Read {} nodes", root.count_nodes());

	// Set up the resource information
	try!(root.fill_in_gantt());

	// Display the gantt chart
	try!(root.display_gantt());

	Ok(())
}


#[cfg(not(test))]
fn main() {

	match do_work() {
		Ok(()) => println!("Complete!"),
		Err(e) => println!("Error: {}", e)
	};
}
