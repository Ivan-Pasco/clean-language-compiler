# 19. AI Integration

Three language constructs help AI-assisted tooling understand a program's provenance: the `spec` statement links a function to its specification document, the `intent` statement describes the function's purpose in plain language, and the file-level `source:` block marks a file as generated from a specification. This chapter defines all three; the tooling that reads them — the framework's MCP server — is specified elsewhere.

Clean Language provides built-in features to support AI-assisted development by linking code to formal specifications and documenting function intent.

### Overview

The AI Integration features enable:
- **Traceability**: Link functions to their specification documents
- **Intent Documentation**: Describe function purpose in natural language
- **Source Attribution**: Mark generated files with their specification source

These features are language-level constructs that enhance code clarity and enable tools to understand the relationship between specifications and implementations.

### AIM-01 — The `spec` statement

The `spec` keyword links a function or method to its specification document.

**Syntax:**
```clean
spec "path/to/specification.spec.cln"
```

**Rules:**
- Can only appear inside function or method bodies
- Must appear before other statements (except `intent`)
- Path is relative to the project root
- Multiple `spec` declarations are allowed (for referencing multiple specs)

**Example:**
```clean
functions:
	number calculateDiscount(number price, number percentage)
		spec "specs/pricing/discount.spec.cln"
		before:
			percentage >= 0 and percentage <= 100
		return price * (percentage / 100)
```

### AIM-02 — The `intent` statement

The `intent` keyword describes a function's purpose in natural language.

**Syntax:**
```clean
intent "Natural language description of function purpose"
```

**Rules:**
- Can only appear inside function or method bodies
- Must appear before other statements (except `spec`)
- Provides human-readable documentation of what the function does
- Multiple `intent` declarations are allowed

**Example:**
```clean
functions:
	void processPayment(number amount, string method)
		intent "Process a payment transaction using the specified payment method"
		spec "specs/payment/process.spec.cln"
		before:
			amount > 0
			["credit", "debit", "paypal"].contains(method)
		// ... implementation
```

### AIM-03 — The `source:` block

The `source:` block marks a file as generated from a specification document.

**Syntax:**
```clean
source:
	spec: "path/to/specification.spec.cln"
	version: "commit-hash-or-version"
```

**Rules:**
- Must appear at the top of the file (before any other declarations)
- Contains two required fields:
  - `spec`: Path to the source specification file
  - `version`: Version identifier (git hash, version number, etc.)
- Indicates the file was generated from a formal specification

**Example:**
```clean
source:
	spec: "specs/payment.spec.cln"
	version: "a3f2c1d"

functions:
	boolean validateCard(string cardNumber)
		intent "Validate credit card number using Luhn algorithm"
		spec "specs/payment.spec.cln"
		before:
			cardNumber.length() >= 13 and cardNumber.length() <= 19
		// ... implementation
```

### Combined Usage Example

Here's a complete example showing all AI integration features together:

```clean
source:
	spec: "specs/authentication.spec.cln"
	version: "2.1.0"

functions:
	boolean authenticateUser(string username, string password)
		intent "Authenticate a user with username and password credentials"
		spec "specs/authentication.spec.cln"
		before:
			username.length() > 0
			password.length() >= 8

		User? stored = User.data.findBy("username", username)
		if stored is none
			return false

		return auth.verifyPassword(password, stored!.passwordHash)

tests:
	"authenticate valid user": authenticateUser("john", "SecurePass123") == true
	"reject invalid password": authenticateUser("john", "WrongPass") == false
```

The lookup goes through the [data library](../02%20components/framework/libraries/04-data.md)'s companion, and the hash comparison through the [auth library](../02%20components/framework/libraries/01-auth.md). Neither writes SQL by hand, and neither calls a host function directly — those are declared once in a library's `host_bridge.cln` ([Libraries Specification §8](../02%20components/framework/09-libraries-specification.md#8-host-bridge-as-typed-host-function-declarations)).

### Use Cases

**1. Specification-Driven Development:**
```clean
functions:
	number calculateTax(number income, string state)
		spec "specs/tax/calculation.spec.cln"
		intent "Calculate state income tax based on tax brackets"
		before:
			income >= 0
			["CA", "NY", "TX", "FL"].contains(state)
		// Implementation follows specification
```

**2. Documentation and Traceability:**
```clean
functions:
	void sendEmail(string to, string subject, string body)
		intent "Send email via SMTP with retry logic and error handling"
		spec "specs/email/smtp.spec.cln"
		before:
			to.contains("@")
			subject.length() > 0
		// Implementation traceable to spec
```

**3. Generated Code Attribution:**
```clean
source:
	spec: "specs/api/rest_endpoints.spec.cln"
	version: "v1.2.3"

// Generated API endpoint handlers
functions:
	string handleGetUser(string userId)
		intent "Handle GET /api/users/:id endpoint"
		spec "specs/api/rest_endpoints.spec.cln"
		// ... implementation
```

### Best Practices

1. **Use relative paths**: Spec paths should be relative to project root
   ```clean
   spec "specs/module/feature.spec.cln"  // ✅ Good
   spec "/absolute/path/spec.cln"         // ❌ Avoid
   ```

2. **Place metadata early**: `spec` and `intent` should come before contract blocks and implementation
   ```clean
   functions:
   	void myFunc()
   		intent "..."    // ✅ Good: appears first
   		spec "..."      // ✅ Good: appears with intent
   		before:
   			x > 0
   		// implementation
   ```

3. **Keep intent concise**: One clear sentence describing the function's purpose
   ```clean
   intent "Calculate compound interest over a given period"  // ✅ Good
   intent "This function does stuff with numbers"            // ❌ Too vague
   ```

4. **Version tracking**: Use meaningful version identifiers in `source:` blocks
   ```clean
   source:
   	spec: "specs/payment.spec.cln"
   	version: "a3f2c1d"    // ✅ Git hash
   ```

### Notes

- These features are metadata and don't affect runtime behavior
- The compiler can use this information for validation and tooling
- Specification files (`.spec.cln`) follow the same Clean Language syntax
- Tools can verify that implementations match their specifications

Numbered error codes for missing spec paths or intent descriptions live in [Semantic Rules](../03%20platform/10-semantic-rules.md) (SYN100, SYN101).

### Tooling

The constructs above are how a program records its provenance. The channel through which an AI assistant or IDE *reads* that provenance — and everything else about a project — is the framework's MCP server. There is exactly one, it belongs to the framework, and it is specified in [10 — MCP Server Architecture](../02%20components/framework/10-mcp-server-architecture.md); the decision behind that arrangement is [ADR-0001](../01%20governance/decisions/0001-single-mcp-server.md). Installing and running it is a Clean Manager command ([Manager](../02%20components/manager/00-manager.md)).

A library describes itself to that server through the `[mcp]` tables of its `library.toml` ([Libraries Specification §5](../02%20components/framework/09-libraries-specification.md)) — the manifest has no separate `[ai]` section.

## Changelog

- 2026-08-01 — Fase 5 (zero-debt pass): Use Cases, Best Practices and Notes marked *Informative*.
- 2026-08-01 — Fase 3/4 (L21): the invented **`[ai]` manifest section removed** — its three keys duplicate `[library] description`, `[mcp.examples]` and `[mcp] instructions` in the real schema ([Libraries Specification §5](../02%20components/framework/09-libraries-specification.md)). The 53-line MCP tooling section replaced by a citation of its home, [10 — MCP Server Architecture](../02%20components/framework/10-mcp-server-architecture.md), and the chapter now cites [ADR-0001](../01%20governance/decisions/0001-single-mcp-server.md), the decision it silently agreed with. The worked example rewritten: raw `database.query("SELECT …")` replaced by the data library's companion, and four `_crypto_*` underscore host names — a rejected form — replaced by the auth library's surface. Rules `AIM-01`..`AIM-03` minted.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Clean Language users linking code to specifications; tool authors and AI assistants reading a program's provenance
- **Rule prefix:** `AIM-`
- **Part of:** [Clean Language Specification — Language](./README.md)
- **References:** [Framework — MCP Server Architecture](../02%20components/framework/10-mcp-server-architecture.md), [Libraries Specification §5](../02%20components/framework/09-libraries-specification.md) (library MCP tables), [ADR-0001](../01%20governance/decisions/0001-single-mcp-server.md)
