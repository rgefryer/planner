#![feature(plugin)]
#![plugin(rocket_codegen)]
extern crate rocket;
extern crate rocket_contrib;
extern crate serde_json;
#[macro_use] extern crate serde_derive;

mod chart;
use std::cell::RefCell;
use std::rc::Rc;
use rocket_contrib::Template;

#[derive(Serialize)]
struct TemplateRow {
    what: String,
    who: String,
    done: String,
    left: String,
    even: bool,
    weeks: Vec<String>
}

#[derive(Serialize)]
struct TemplateContext {
    weeks: Vec<String>,
    rows: Vec<TemplateRow>
}


#[cfg(not(test))]
#[get("/")]
fn index() -> Template {

    let mut context = TemplateContext {
        weeks: vec!["1", "2", "3", "4", "5"].iter().map(|s| s.to_string()).collect(),
        rows: Vec::new()
    };
    for ix in 1 .. 10 {
        context.rows.push(TemplateRow {
            what: "Big important task".to_string(),
            who: "rf".to_string(),
            done: "10.5".to_string(),
            left: "20".to_string(),
            even: (ix % 2) == 0,
            weeks: vec![3f32, 2.5f32, 0f32, 1.25f32, 8f32].iter().map(|s| format!("{:.2}", s).replace(".0", "&nbsp;&nbsp;").replace("0", "&nbsp;")).collect()
        });
    }

    Template::render("index", &context)
}

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
    rocket::ignite().mount("/", routes![index]).launch();

}

