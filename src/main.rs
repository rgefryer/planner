mod chart;

use std::cell::RefCell;
use std::rc::Rc;

#[cfg(not(test))]
fn main() {
    match chart::read_config(&("config.txt".to_string())) {
    	Ok(mut f) => {
			let rc_root = Rc::new(RefCell::new(chart::ConfigNode::new("root", 0, 0)));

			// And read the config file
			let mut root = rc_root.borrow_mut();
			root.consume_config(Some(&rc_root), &mut f).unwrap();

    		println!("Read {} nodes", root.count_nodes());
    	},
    	Err(e) => println!("{}", e.to_string()),
    };
}
