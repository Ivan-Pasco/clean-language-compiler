# 14. Classes and Objects

Classes are Clean's home for domain behavior and long-lived state. Data plus the methods that act on it live together in a class; entry blocks like `start:` delegate to classes rather than implementing logic directly. Clean does not force everything to be object-oriented, but for anything that has a lifecycle or evolves over time — a user, a shopping cart, a render loop — a class is the natural shape. This chapter defines the syntax, the rules for what may appear inside a class body, and the two constrained forms of cross-object access (capabilities and companion state) that keep classes composable without global mutation.

### CLS-01 — Class definition

All class methods must be declared within a `functions:` block:

```clean
class Point
	integer x
	integer y

	constructor(integer initialX, integer initialY)
		x = initialX
		y = initialY

	functions:
		number distanceFromOrigin()
			return math.sqrt(x * x + y * y)

		void move(integer dx, integer dy)
			x = x + dx
			y = y + dy
```

### Generic Classes with `any`

Clean Language uses `any` for generic class fields and methods:

```clean
class Container
	any value

	constructor(any initialValue)
		value = initialValue

	functions:
		any get()
			return value

		void set(any newValue)
			value = newValue
```

### CLS-02 — Inheritance

Clean Language supports single inheritance using the `is` keyword. Child classes inherit all public fields and methods from their parent class.

```clean
class Shape
	string color
	
	constructor(string colorParam)
		color = colorParam          // Implicit context - no 'this' needed
	
	functions:
		string getColor()
			return color            // Direct field access

class Circle is Shape
	number radius
	
	constructor(string colorParam, number radiusParam)
		base(colorParam)            // Call parent constructor with 'base'
		radius = radiusParam        // Implicit context
	
	functions:
		number area()
			return 3.14159 * radius * radius
		
		string getInfo()
			return color + " circle"    // Access inherited field directly
```

#### Inheritance Features

- **Syntax**: Use `class Child is Parent` to inherit from a parent class
- **Base Constructor**: Use `base(args...)` to call the parent constructor
- **Implicit Context**: No need for `this` or `self` - fields are directly accessible
- **Name Safety**: Parameters must have different names than fields to prevent conflicts
- **Method Inheritance**: Child classes inherit all public methods from parent classes
- **Field Inheritance**: Child classes inherit all public fields from parent classes
- **Method Overriding**: Child classes can override parent methods by defining methods with the same name

#### Implicit Context Rules

Clean Language uses implicit context for accessing class fields:

- ✅ `color = colorParam` (field assignment — implicit `this`)
- ✅ `return color` (field access — implicit `this`)
- ✅ `radius = radiusParam` (works in child classes too)
- ✅ `this.render()` (explicit self-method call)
- ✅ `this.name` (explicit field access — equivalent to just `name`)
- ❌ Parameter names cannot match field names (compiler enforced)

Fields can be accessed directly by name (implicit `this`) or with explicit `this.field`. The `this` keyword is available inside all class methods and refers to the current instance. Explicit `this` is useful only for self-method calls.

### CLS-03 — Capabilities are contracts without bodies

A **capability** describes something a class can do. It is a named contract of methods that any class may claim to fulfil. When a class declares `can Draw`, it promises to provide every method the `Draw` capability requires, and becomes usable anywhere a value of type `Draw` is expected.

Capabilities solve the "several things that play the same role" problem: instead of writing one function per concrete type, you write one function that accepts the capability.

#### Declaring a capability

```clean
can Draw:
	draw()

can Describe:
	describe() -> string
	tag() -> string
```

Each item in a `can:` block is a method signature — a name, parameter list, and optional return type. **Capabilities are pure contracts: signatures only, never bodies.** They describe *what* a class must be able to do, not *how* it does it. Every method a capability declares is required — every class that claims the capability must implement every method it declares. There are no default implementations at the capability level: if two classes need the same behavior, factor it into a shared function or a parent class, not the capability.

A `can:` block that contains a method body is [`SEM014`](../03%20platform/09-error-codes.md#32-semantic-codes-sem).

#### Claiming a capability

A class claims one or more capabilities using `can` after its (optional) `is` clause:

```clean
class Circle is Shape can Draw, Describe
	public:
		number radius

	functions:
		public:
			draw()
				print("circle")
			describe() -> string
				return "circle"
			tag() -> string
				return "[circle]"
```

Order: `class Name [is Parent] [can C1, C2, ...]`. When both clauses are present, `is` comes first.

Capabilities are **nominal**: a class that happens to have a `draw()` method does *not* have the `Draw` capability. It must say `can Draw`.

**Naming convention.** Because the keyword is `can`, the capability name should read as an action the class can perform. Prefer bare verbs: `can Draw`, `can Print`, `can Close`, `can Serialize`, `can Compare`, `can Iterate`. `class Circle can Draw` reads naturally in English; adjective forms like `can Drawable` do not. This is a stylistic convention (not enforced by the compiler) but every example in this specification follows it.

#### Using a capability as a type

Capability names may appear anywhere a type is expected — parameters, return types, variable declarations, and generic type arguments:

```clean
functions:
	render(Draw thing)
		thing.draw()

	renderAll(list<Draw> things)
		iterate item in things
			render(item)
```

At the call site of a capability-typed value, method dispatch is **dynamic**: the actual class of the value at runtime determines which implementation runs. This lets one list hold several different classes that share the same capability.

What is observable is the dispatch itself: the same call on two values of different classes runs two different implementations, and which one runs is decided by the value, not by the declared type. How that is represented in memory is not part of this contract — object layout is specified once in [Platform 03 — Memory Model](../03%20platform/03-memory-model.md).

#### Interaction with inheritance

- A class inherits every capability its parent has. If `Shape can Draw`, then `Circle is Shape` also `can Draw` — no need to redeclare.
- A child class may claim additional capabilities beyond its parent's.
- A capability method implemented on the parent satisfies the capability for the child, unless the child overrides it.

#### What capabilities are not

- Not classes: you cannot instantiate a capability.
- Not structural: a class must explicitly claim the capability.
- Not carriers of behavior: a `can:` block contains signatures only. It cannot include method bodies, fields, or state. Share behavior through a parent class or a top-level function, not through the capability.
- Not generic in v1: `can Comparable<T>` is not yet supported.
- Not composable in v1: one capability cannot extend another.

### CLS-04 — Object creation

```clean
start:
	// Create objects
	Point point = Point(3, 4)
	Circle circle = Circle("red", 5.0)

	// Call methods (parentheses required)
	number distance = point.distanceFromOrigin()
	point.move(1, -2)

	// Access properties
	integer xCoord = point.x
	string color = circle.color
```

### Static Methods

You can call class methods directly on the class name if they don't use instance fields:

```clean
class MathUtils
	functions:
		number add(number a, number b)
			return a + b
		
		number max(number a, number b)
			if a > b
				return a
			return b

class DatabaseService
	functions:
		boolean connect(string url)
			// implementation that doesn't use instance fields
			return true
		
		User findUser(integer id)
			// implementation that doesn't use instance fields
			return User.loadFromDatabase(id)

// Static method calls - ClassName.method()
start:
	number result = MathUtils.add(5.0, 3.0)
	number maximum = MathUtils.max(10.0, 7.5)
	boolean connected = DatabaseService.connect("mysql://localhost")
	User user = DatabaseService.findUser(42)
```

**Rules for Static Methods:**
- Use `ClassName.method()` syntax for static calls
- Only allowed if the method doesn't access instance fields (`this.field`)
- All methods must be in `functions:` blocks
- Method calls require parentheses: `MathUtils.add()` not `MathUtils.add`
- Ideal for helpers, services, utilities, and database access functions

**Example - Mixed Static and Instance Methods:**
```clean
class User
	string name
	integer age
	
	constructor(string userName, integer userAge)
		name = userName
		age = userAge
	
	functions:
		// Instance method - accesses fields
		string getInfo()
			return "User: {name}, Age: {age}"
		
		// Static method - no field access
		boolean isValidAge(integer age)
			return age >= 0 and age <= 150

// Usage
start:
	User user = User("Alice", 25)
	string info = user.getInfo()                    // Instance method call
	boolean valid = User.isValidAge(30)             // Static method call
```

### CLS-05 — Companion access

Field access on a class name — written `ClassName.fieldName` rather than `instance.fieldName` — resolves to the field's declared type used as a namespace. Method calls on that expression dispatch to the type's static methods.

This is a single rule with broad consequences: any class field named `data`, `queries`, `factory`, `config`, `defaults`, `cache`, or anything else becomes a **companion** — a separate type reachable through the class without needing an instance.

**The rule:**

Given `class Outer` with a field `Inner fieldName`:

- `instance.fieldName` is normal instance field access — returns the `Inner` value held in that instance.
- `Outer.fieldName` is companion access — returns the *type* `Inner` used as a namespace. Method calls on the result dispatch to `Inner`'s static methods.

**Example:**

```clean
class Rectangle
	number width
	number height
	RectangleFactory factory

class RectangleFactory
	functions:
		Rectangle fromCorners(Point a, Point b)
			return Rectangle(math.abs(b.x - a.x), math.abs(b.y - a.y))

		Rectangle unit()
			return Rectangle(1.0, 1.0)

// Instance-side access — reaches the field's value
start:
	Rectangle r = Rectangle(3.0, 4.0)
	RectangleFactory f = r.factory     // regular field access

// Class-side access — reaches the field's type as a namespace
start:
	Rectangle unit = Rectangle.factory.unit()
	Rectangle box = Rectangle.factory.fromCorners(Point(0, 0), Point(10, 5))
```

Nothing about the mechanism is specific to any domain. The compiler recognizes exactly one rule: when the receiver is a class name and the next token is a field of that class, the resulting expression has *type-as-namespace* meaning rather than value meaning.

**Uses:**

| Pattern | Example |
|---------|---------|
| **Factories** | `Rectangle.factory.fromCorners(a, b)`, `User.factory.fromJson(str)` |
| **Named constants** | `Color.defaults.red`, `Vector.constants.zero`, `Http.status.ok` |
| **Repositories / queries** | `User.data.findById(1)`, `Order.queries.recent()` |
| **Registries** | `Widget.registry.all()`, `EventBus.subscribers.count()` |
| **Type-level utilities** | `Duration.parse.iso8601(str)`, `Email.validate.strict(input)` |

Each of these is the same rule applied to a different domain. There is no `factory` keyword, no `constants` keyword, no `registry` keyword — every companion is just an ordinary class held in a field.

**Rules for Companion Access:**

- The receiver must be a class name (`Rectangle`), not an instance (`r`).
- The field named after the class must exist in the class definition, with a declared type.
- Only static methods on the field's type are reachable through companion access. Instance methods on the companion type require an actual instance.
- Multiple companions per class are allowed — declare multiple fields.
- The companion's type does not need to know it is being used as a companion. It is an ordinary class.
- A class may claim capabilities on its companion type independently from the outer class. `class UserData can Persist` does not require `class User can Persist`.

**Interaction with capabilities:**

Capabilities are declared on whichever class the behavior belongs to — the outer class, the companion, or both. The two are independent.

```clean
class User                          // no infrastructure capability
	UserData data
	string email

class UserData can Persist          // persistence lives with the data companion
	integer id
	string email
	string passwordHash

	functions:
		User? findById(integer id)         // static — reachable via User.data.findById(...)
		User? findByEmail(string email)    // static — reachable via User.data.findByEmail(...)

// Call sites:
start:
	User? u = User.data.findByEmail("alice@example.com")  // read via companion access
	User? u2 = User.data.findById(1)
	Database.save(u)                                       // write via the Database facade (see the data library)
	Database.delete(u)
```

The entity class stays focused on domain concerns. Infrastructure capabilities (`Persist`, `Cacheable`, `Auditable`) live on the companion where the infrastructure state and methods already live. Testing an entity does not drag in the database; testing the companion does.

Companion access covers *reads* and *type-level queries* — anywhere `TypeName.field.method(...)` makes sense as a call on the field's type. Writes and other cross-cutting operations (transactions, batch saves) may go through a facade the library provides (`Database.save`, `Database.saveAll`, `transaction:`), independent of companion access.

**Comparison to related mechanisms:**

- **Static methods** on the class itself (previous section) reach class-level behavior *of the class*. Companion access reaches class-level behavior *of a companion type*. Use static methods for utilities that logically belong to the class (`User.isValidAge`); use companion access when the concern belongs to a different type (`User.data.findById`).
- **Instance field access** always reaches the field's value. Companion access reaches the field's type. Same syntactic shape (`Outer.field`), disambiguated by whether `Outer` is an instance or a class name.
- **Namespaces** like `math.sqrt(...)` and `string.concat(...)` are top-level namespaces that don't belong to any class. Companion access is per-class: `User.data` and `Order.data` reach different companion types.

### Design Philosophy: Flexible Organization

The rest of this section is guidance, not normative.

Clean Language supports both class-based organization and top-level functions, providing flexibility for different coding styles and project needs:

#### Class-Based Organization (Recommended for complex projects)
- **Better code organization**: Related functionality is grouped together
- **Namespace management**: No global function name conflicts  
- **Consistent syntax**: All method calls use the same `Class.method()` or `object.method()` pattern
- **Extensibility**: Easy to add related methods to existing classes

```clean
class Calculator
	functions:
		number calculateTax(number amount)
			return amount * 0.15
		
		string formatResult(number value)
			return "Result: " + value.toString()
```

#### Top-Level Functions (Suitable for simpler projects)
- **Direct approach**: Functions can be declared directly in `functions:` blocks
- **Simplicity**: No need for class wrapper when functionality is standalone
- **Scripting style**: a `start:` block with top-level `functions:`, and no classes

```clean
start:
	number tax = calculateTax(100.0)
	string result = formatResult(tax)
	print(result)

functions:
	number calculateTax(number amount)
		return amount * 0.15

	string formatResult(number value)
		return "Result: " + value.toString()
```

**Both approaches are valid and can be mixed within the same program.** The choice depends on project complexity and developer preference.

## Changelog

- 2026-08-17 — Erratum: the `Container` example wrote `constructor(any value)` — bodyless, with the parameter named after the field `value` — violating [`CLASS010`](../03%20platform/09-error-codes.md#35-class-codes-class) (the rule the 2026-08-01 pass below registered, and which already corrected `Point` and `User` for the same defect) and implying an implicit parameter→field assignment no chapter specifies. There is no implicit field assignment: the constructor now names its parameter `initialValue` and assigns explicitly; a bodyless constructor simply runs an empty body. Found by the compiler's Milestone 4 (`clean-language-compiler/docs/DISCOVERIES-M4.md`, item 13).
- 2026-08-01 — Fase 5 (zero-debt pass): the `Point` and `User` examples no longer name a constructor parameter after a field — they violated [`CLASS010`](../03%20platform/09-error-codes.md#35-class-codes-class), the rule this pass registered, and relied on an implicit field assignment no chapter specifies. The 4-byte class-id header and dispatch switch removed (object layout is [Platform 03](../03%20platform/03-memory-model.md)'s). `can:` bodies now cite [`SEM014`](../03%20platform/09-error-codes.md#32-semantic-codes-sem).
- 2026-08-01 — Fase 4: rules `CLS-01`..`CLS-05` minted; prefix `CLS-` registered. The chapter is the declared home of two glossary terms (Capability, Companion) and is cited by twelve documents across three trees, none of which could cite a rule ID before.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Clean Language users learning classes, capabilities, and companion access; library authors declaring domain types
- **Rule prefix:** `CLS-`
- **Part of:** [Clean Language Specification — Language](./README.md)
- **References:** [Platform 03 — Memory Model](../03%20platform/03-memory-model.md) (object layout), [Platform 09 — Error Codes](../03%20platform/09-error-codes.md) (CLASS range), [Glossary](../01%20governance/06-glossary.md) (Capability, Companion)
- **Satisfies:** LANG-01, LANG-04
