extern crate augeas;

use augeas::Augeas;

fn main() {
  let aug = Augeas::new("", "");
  let username = "root";

  println!("Infos about '{}':", username);

  let info_nodes = aug.matches("etc/passwd/root/*");

  for node in info_nodes.iter() {
    let label = aug.label(node.as_slice());
    let value = aug.get(node.as_slice());

    match (label, value) {
      (Some(label), Some(value)) => println!("{} = {}", label, value),
      _ => {}
    }
  }
}