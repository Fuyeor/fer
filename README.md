**The Fer Programming Language** is a high-performance systems programming language designed for long-term maintainability, developer efficiency, and ease of code generation. It is built to unify and replace our fragmented multi-language backend stacks, providing a consistent and robust foundation for modern infrastructure.

> [!IMPORTANT]
> This is the specification for the **Fer Programming Language**. For information regarding **Natural Language Fer**, please visit [fer.fuyeor.com](https://fer.fuyeor.com).

## Syntax Reference

### 1. Module Imports

Fer uses a strict module system. Relative paths using `../` are prohibited to ensure project structure clarity.

- **Standard Library**: `{ get post } = @fer/http`
- **Root-relative (Internal)**: `{ check-username-availability } = @/utils/username`
- **Relative (Current Directory)**: `{ create-user } = ./repository`
- **Renaming (Aliasing)**: `{ get post Http = HttpClient } = @fer/http` (Renames `Http` to `HttpClient`)

### 2. Comments

- `// Single-line comment`
- `/// Documentation comment`
- `/* Multi-line comment */`

### 3. Constants and Assignments

In Fer, all definitions are **immutable constants** by default. There are no variables, ensuring thread safety and predictability.

- `define-var = "variable"` (Type inference, equivalent to `const` in JS)
- `x = 0` (Inferred as `i8`)
- **Statement Terminator**: Newline (`\n`)

### 4. Arrays

- **Explicit Definition**: `array = [123 456 789]`
- **Auto-completion Support**: While implicit arrays (space-separated) were deprecated in `v0.0.1` to prevent ambiguity, the IDE plugin provides smart completion to streamline writing.

### 5. Destructuring

`{ const1 const2 const3 = expr } = object`

### 6. Condition Expressions

A `condition` is an expression wrapped in `()` that returns a boolean.

- **Strings**: `contains`, `starts`, `ends`, `equals`, `matches` (Regex)
- **Numbers**: `less` (`<`), `more` (`>`), `least` (`>=`), `most` (`<=`), `equals`
- **Arrays**: `in` (e.g., `user.relationship in [follower friend]`)
- **Logic**: `and`, `or`, `xor`, `not` (or `!(...)`)

Example: `(comment.content matches \btx(|et|t|.*)\b) or (user.reputations less 200)`

### 7. Match Expressions

The `match` expression provides a powerful way to perform pattern-based assignments.

```fer
// Assign the result of matching 'constant2' to 'constant1'
constant1 = constant2 {
  // Syntax sugar for direct value matching
  value { return-value }
  operator value { return-value }
  // Default branch (equivalent to 'else' or 'switch default')
  { default-value }
}

// Example:
age = 20
print(age {
  < 18 { status = `minor` }
  > 60 { status = `old` }
  { status = `adult` }
}) // Outputs: "adult"

// Combining with Condition Expressions
result = ((constant1 matches regex) or (constant2 contains `xxx`)) {
  true { `Matched` }
  { `Not Matched` }
}
```

### 8. Functions

To ensure maintainability and ease of refactoring, functions require explicit parameter and return types.
*   **Named Parameters**: For functions with 2 or more arguments, **named parameters are mandatory**. Positional arguments are forbidden to prevent logic errors during updates.

```fer
// Named Function
authenticate(user: string, token: string) -> Bool {
  // Function body
}

// Anonymous / High-order Function
callback = (req: Request) -> Response {
  handle(req)
}
```

### 9. String Handling

Fer exclusively uses backticks (`` ` ``) for strings, supporting interpolation and multi-line formatting.

```fer
name = `Fuyeor`
// String Interpolation
message = `Hello, {name}!`
// The {} block is an expression; it evaluates the content and returns it.
calculate-message = `1 + 1 = { 1 + 1 }`

// Multi-line strings with smart indentation trimming
multiple1 = `
  {message}
  This is a string
  that spans multiple
  lines easily.
  `

// Line continuation using backslash
multiple2 = `This is a string \
  that spans multiple \
  lines easily.`
```

### 10. Data Structures (Structs & Enums)

Fer uses `=` for member definitions to minimize "syntactic friction."

```fer
// Define an Enum
Gender = enum {
  ai female male nonbinary
}

// Define a Struct
User = struct {
  id     = i64
  name   = string
  gender = Gender
}
```