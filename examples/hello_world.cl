// A simple hello world program in the Clean language

// The start function is the entry point
start()
    // Print a welcome message
    print("Hello, World from Clean Language!")
    
    // Basic arithmetic operations
    x = 10
    y = 5
    sum = x + y
    diff = x - y
    product = x * y
    quotient = x / y
    
    // Print the results
    print("Sum: " + sum)
    print("Difference: " + diff)
    print("Product: " + product)
    print("Quotient: " + quotient)
    
    // Conditional statement
    if x > y
        print("x is greater than y")
    else
        print("x is less than or equal to y")
    
    // Loop - iterate from 1 to 5
    iterate i from 1 to 5
        print("Iteration: " + i)
    
    // Define and call a function
    result = calculate(x, y)
    print("Calculated result: " + result)

// A custom function with parameters
function calculate(a, b)
    return a * b + a - b 