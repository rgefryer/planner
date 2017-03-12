
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LineNode {
	pub line_num: u32,
	pub indent: u32,
	pub name: String
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LineAttribute {
	pub key: String,
	pub value: String
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Line {
	Node(LineNode),
	Attribute(LineAttribute)    
}

impl Line {
	pub fn new_node_line(line_num: u32, indent: u32, name: &str) -> Line {
		Line::Node(LineNode {line_num: line_num, indent: indent, name: name.to_string()})
	}

	pub fn new_attribute_line(key: &str, value: &str) -> Line {
		Line::Attribute(LineAttribute {key: key.to_string(), value: value.to_string()})
	}
}

pub struct ConfigLines {
	lines: Vec<Line>,
	pos: usize
}

impl ConfigLines {

	pub fn new() -> ConfigLines {
		ConfigLines { lines: Vec::new(), pos: 0 }
	}

	pub fn add_line(&mut self, line: Line) {
		self.lines.push(line);
	}

	pub fn peek_line(&self) -> Option<Line> {
		if self.lines.len() > self.pos {
			Some(self.lines[self.pos].clone())
		}
		else {
			None
		}
	}

	pub fn get_line(&mut self) -> Option<Line> {
		if self.lines.len() > self.pos {
			self.pos += 1;
			Some(self.lines[self.pos-1].clone())
		}
		else {
			None
		}
	}
}
