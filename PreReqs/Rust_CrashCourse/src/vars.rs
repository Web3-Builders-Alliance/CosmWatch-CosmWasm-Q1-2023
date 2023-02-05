// Variables hold primitive data or references to data
// Variables are immutable by default
// Rust is a block-scoped language

pub fn run() {
    let name = "Max";
    let mut age = 25;

    age = 26;

    println!("My name is {} and I am {}", name, age);

    // Define constants
    const ID: i32 = 1;
    println!("ID: {}", ID);

    // Assign multiple vars
    let (my_name, my_age) = ("Max", 25);
    println!("{} is {}", my_name, my_age);

}