// Complex example demonstrating the Clean Language features
// This example includes classes, error handling, functions, loops, and more

// Point class for 2D coordinates
class Point
    x: float
    y: float
    
    constructor(x, y)
        this.x = x
        this.y = y
    
    function distance(other: Point): float
        dx = this.x - other.x
        dy = this.y - other.y
        return math.sqrt(dx * dx + dy * dy)
    
    function toString(): string
        return "(" + this.x + ", " + this.y + ")"

// Shape interface with area and perimeter methods
class Shape
    function area(): float
        error("Area method not implemented")
    
    function perimeter(): float
        error("Perimeter method not implemented")
    
    function toString(): string
        return "Shape"

// Circle implementation of Shape
class Circle extends Shape
    center: Point
    radius: float
    
    constructor(center, radius)
        this.center = center
        this.radius = radius
    
    function area(): float
        return math.PI * this.radius * this.radius
    
    function perimeter(): float
        return 2 * math.PI * this.radius
    
    function toString(): string
        return "Circle(center=" + this.center.toString() + ", radius=" + this.radius + ")"

// Rectangle implementation of Shape
class Rectangle extends Shape
    topLeft: Point
    width: float
    height: float
    
    constructor(topLeft, width, height)
        this.topLeft = topLeft
        this.width = width
        this.height = height
    
    function area(): float
        return this.width * this.height
    
    function perimeter(): float
        return 2 * (this.width + this.height)
    
    function toString(): string
        return "Rectangle(topLeft=" + this.topLeft.toString() + 
               ", width=" + this.width + ", height=" + this.height + ")"

// Function to safely divide numbers with error handling
function safeDivide(a, b)
    if b == 0
        error("Division by zero")
    return a / b

// Function to create an array of random points
function generateRandomPoints(count): array
    points = []
    iterate i from 0 to count - 1
        x = math.random() * 100
        y = math.random() * 100
        points[i] = Point(x, y)
    return points

// Function to find the closest pair of points in an array
function findClosestPair(points: array): array
    if array.length(points) < 2
        error("Need at least 2 points")
    
    minDistance = 1000000
    p1 = null
    p2 = null
    
    iterate i from 0 to array.length(points) - 2
        iterate j from i + 1 to array.length(points) - 1
            point1 = points[i]
            point2 = points[j]
            d = point1.distance(point2)
            
            if d < minDistance
                minDistance = d
                p1 = point1
                p2 = point2
    
    return [p1, p2, minDistance]

// Main program entry point
start()
    printl("Clean Language Complex Example")
    printl("==============================")
    
    // Basic variables and operations
    printl("\n1. Basic operations:")
    a = 10
    b = 3
    printl("a = " + a + ", b = " + b)
    printl("a + b = " + (a + b))
    printl("a - b = " + (a - b))
    printl("a * b = " + (a * b))
    
    // Division with error handling
    printl("\n2. Error handling with division:")
    
    // Safe case
    result = safeDivide(a, b)
    printl("a / b = " + result)
    
    // Error case with handler
    result = safeDivide(a, 0) onError:
        printl("Caught division by zero error")
        return "Error: cannot divide by zero"
    printl("a / 0 = " + result)
    
    // Classes and objects
    printl("\n3. Classes and objects:")
    p1 = Point(3, 4)
    p2 = Point(6, 8)
    printl("Point 1: " + p1.toString())
    printl("Point 2: " + p2.toString())
    printl("Distance: " + p1.distance(p2))
    
    // Shapes
    printl("\n4. Inheritance and polymorphism:")
    circle = Circle(Point(5, 5), 10)
    rectangle = Rectangle(Point(0, 0), 4, 3)
    
    printl(circle.toString())
    printl("Area: " + circle.area())
    printl("Perimeter: " + circle.perimeter())
    
    printl(rectangle.toString())
    printl("Area: " + rectangle.area())
    printl("Perimeter: " + rectangle.perimeter())
    
    // Arrays and iteration
    printl("\n5. Arrays and iteration:")
    numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    
    // Sum using iterate
    sum = 0
    iterate num in numbers
        sum = sum + num
    printl("Sum of numbers: " + sum)
    
    // Filter even numbers
    evens = []
    evenIndex = 0
    iterate num in numbers
        if num % 2 == 0
            evens[evenIndex] = num
            evenIndex = evenIndex + 1
    
    printl("Even numbers: " + evens.toString())
    
    // Closest pair of points
    printl("\n6. Finding closest pair of points:")
    points = [
        Point(1, 1),
        Point(2, 3),
        Point(5, 4),
        Point(9, 6),
        Point(3, 7)
    ]
    
    result = findClosestPair(points) onError:
        printl("Error finding closest pair")
        return [null, null, -1]
    
    if result[0] != null
        printl("Closest points: " + result[0].toString() + " and " + result[1].toString())
        printl("Distance: " + result[2])
    
    // Conditionals
    printl("\n7. Conditionals:")
    temperature = 75
    
    if temperature > 90
        printl("It's hot outside!")
    else if temperature > 70
        printl("It's warm outside!")
    else if temperature > 50
        printl("It's cool outside!")
    else
        printl("It's cold outside!")
    
    printl("\nComplex example complete!") 