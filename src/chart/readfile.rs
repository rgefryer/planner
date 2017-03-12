use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;
use super::file::*;

pub fn read_config(filename: &str) -> Result<ConfigLines, String> {

	let f = match File::open(filename) {
		Ok(f) => f,
		Err(err) => return Err(err.to_string()),
	};

	let mut file_data = ConfigLines::new();
	let mut line_num = 0;

	let reader = BufReader::new(f);
	for line_rc in reader.lines() {

		line_num += 1;
		match line_rc {
			Ok(line) => {
				try!(process_line(&line, &mut file_data, line_num));
			},
			Err(err) => return Err(err.to_string()),
		};
	}

	Ok(file_data)
}

fn process_line(input_line: &str, 
				file_data: &mut ConfigLines, 
				line_num: u32) -> Result<(), String> {

	let mut line = input_line;

	// Discard trailing comments
	match line.find('#') {
		None => {},
		Some(ix) => {
			line = &line[0..ix];
		},
	};

	// Trim the RHS
	line = line.trim_right();

	// Get the length
	let len_with_indent = line.len();

	// Trim the LHS
	line = line.trim_left();

	// Get the indent
	let indent = len_with_indent - line.len();

	// Discard empty lines
	if line.len() == 0 {
		return Ok(());
	}

	// Work out if this is a node or an attribute.
	let node = match line.find("- ") {
		Some(0) => false,
		_ => true,
	};

	// If new node, write note line
	if node {
		file_data.add_line(Line::new_node_line(line_num, (indent + 1) as u32, line));
	}

	// Else if attribute, splt the attribute and values and 
	// write attribute line
	else {
		line = line[2 ..].trim_left();
		match line.find(':') {
			Some(0) => { return Err("Attribute with no key".to_string()) }
			Some(pos) => {
				let attr_name = line[.. pos].trim();
				let attr_val = line[pos+1 ..].trim();

				file_data.add_line(Line::new_attribute_line(attr_name, attr_val));
			}
			None => { return Err("Attribute with no value".to_string())}
		};
	}

	Ok(())
}
