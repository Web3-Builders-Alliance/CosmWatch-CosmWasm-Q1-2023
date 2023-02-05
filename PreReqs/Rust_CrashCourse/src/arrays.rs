// Arrays - Fixed list where elements are the same data types

use std::mem;

pub fn run() {
    let mut numbers: [i32; 4] = [1,2,3,4];

    // Re-assign a value
    numbers[2] = 25;

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
}