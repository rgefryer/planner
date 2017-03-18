#[cfg(not(test))]
use rocket;

#[cfg(not(test))]
use rocket_contrib::Template;

#[cfg(not(test))]
use super::generate_chart_nodes;

#[cfg(not(test))]
use super::ConfigNode;


#[derive(Serialize)]
pub struct TemplateRow {
    what: String,
    who: String,
    done: String,
    left: String,
    even: bool,
    cells: Vec<String>
}

impl TemplateRow {
    pub fn new(indent: u32, name: &str) -> TemplateRow {
        TemplateRow {
            what: format!("{}{}", 
                          format!("{:width$}", " ", width = (indent*3) as usize), 
                          name).replace(" ", "&nbsp;"),
            who: "".to_string(),
            done: " ".to_string(),
            left: " ".to_string(),
            even: false,
            cells: Vec::new()
        }
    }

    pub fn set_who(&mut self, who: &str) {
        self.who = who.to_string();
    }

    fn format_f32(val: f32) -> String {
        if val < 0.01 {
            String::new()
        } else {
            format!("{:.2}", val).replace(".00", "&nbsp;&nbsp;&nbsp;")
                                  .replace(".50", ".5&nbsp;")
        }
    }

    pub fn add_cell(&mut self,  val: f32) {
        self.cells.push(TemplateRow::format_f32(val));
    }

    pub fn set_done(&mut self, done: f32) {
        self.done = TemplateRow::format_f32(done);
    }

    pub fn set_left(&mut self, left: f32) {
        self.left = TemplateRow::format_f32(left);
    }
}

#[derive(Serialize)]
pub struct TemplateContext {
    cell_headers: Vec<String>,
    rows: Vec<TemplateRow>
}

 impl TemplateContext {
    pub fn new(cells: u32) -> TemplateContext {
        TemplateContext {
            cell_headers: (1 .. cells+1).map(|s| format!("{}", s))
                                        .collect(),
            rows: Vec::new()
        }
    }

    pub fn add_row(&mut self, mut row: TemplateRow) {
        row.even = self.rows.len() % 2 == 1;
        self.rows.push(row);
    }
 }


#[cfg(not(test))]
fn generate_chart_html(root: &mut ConfigNode) -> Result<Template, String> {

    let weeks: u32 = try!(root.get_config_val("weeks", None));
    let mut context = TemplateContext::new(weeks);
    try!(root.display_gantt(&mut context));

    Ok(Template::render("index", &context))
}


#[derive(Serialize)]
pub struct ErrorTemplate {
    error: String,
}


#[cfg(not(test))]
fn generate_error_html(err: &str) -> Template {
    Template::render("err", &ErrorTemplate { error: err.to_string() })
}

#[cfg(not(test))]
#[get("/")]
fn index() -> Template {

//pub fn generate_chart_nodes() -> Result<Rc<RefCell<ConfigNode>>, String> {


    match generate_chart_nodes() {
        Ok(rc_root) => {
            let mut root = rc_root.borrow_mut();
            match generate_chart_html(&mut root) {
                Ok(template) => {
                    return template;
                },
                Err(e) => {
                    return generate_error_html(&e);
                }
            }
        },
        Err(e) => {
            return generate_error_html(&e);
        }
    };
}

#[cfg(not(test))]
pub fn serve_web() {
    rocket::ignite().mount("/", routes![index]).launch();
}