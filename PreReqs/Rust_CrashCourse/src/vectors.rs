// Vectors - Resizable arrays

use std::mem;

pub fn run() {
    let mut numbers: Vec<i32> = vec![1,2,3,4];

    // Re-assign a value
    numbers[2] = 25;

    // Add on to vector
    numbers.push(5);
    numbers.push(6);

    // Pop/remove last value
    numbers.pop();

    println!("{:?}", numbers);

    // Get single val 
    println!("Single value: {:?}", numbers[0]);
    
    // Get array length
    println!("Array Length: {}", numbers.len());
    
    // Arrays are stack allocated
    println!("Array occupies {} bytes", mem::size_of_val(&numbers));

    // Get slice 
    let slice: &[i32] = &numbers[0..3];
    println!("Slice: {:?}", slice);

    // Loop through vector values
    for x in numbers.iter() {
        println!("Number: {}", x);
    }
    
    // Loop and mutate values
    for x in numbers.iter_mut() {
        *x *= 2;
    }
    
    println!("Number: {:?}", numbers);
}