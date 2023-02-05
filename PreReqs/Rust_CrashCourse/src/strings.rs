// Primitive str = Immutable fixed-length string somewhere in memory
// String = Growable, heap-allocated data structure - Use when you need to modify or own string data

pub fn run() {
    // Immutable str, fixed length
    let _immutable_hello = "Hello ";
    
    // Mutable String, variable length
    let mut mutable_hello = String::from("Hello ");

    // Get length
    println!("Length: {}", mutable_hello.len());
    
    // Push char
    mutable_hello.push('W');

    // Push string
    mutable_hello.push_str("orld");

    // Capacity in bytes
    println!("Capacity: {}", mutable_hello.capacity());

    // Check if empty
    println!("Is empty: {}", mutable_hello.is_empty());

    // Contains
    println!("Contains 'World' {}", mutable_hello.contains("World"));

    // Contains
    println!("Replace {}", mutable_hello.replace("World", "There"));

    // Loop through string by whitespace
    for word in mutable_hello.split_whitespace() {
        println!("{}", word);
    }

    // Create string with capacity
    let mut s = String::with_capacity(10);
    s.push('a');
    s.push('b');

    // Assertion testing
    assert_eq!(s.len(), 2);
    assert_eq!(s.capacity(), 10);

    println!("{}", s);
}