# Ferrite Standard Library

Ferrite natively includes many builtins inside the core evaluator and comes bundled with a powerful statically embedded module system for advanced routines.

## System Imports

Using the `import` statement checks locally first, and if not found, automatically loads the embedded standard library right from the compiler's own memory. The `.fe` extension is optional.

```ferrite
import "std/mathutils";
import "std/strings";
import "std/collections";
import "std/functional";
```

---

## The Embedded Official Modules

### `mathutils`
* **`square(x)`** - Squares the value.
* **`cube(x)`** - Cubes the value.
* **`hyp(a, b)`** - Hypotenuse `sqrt(a^2 + b^2)`.
* **`clamp(val, min, max)`** - Clamps a value within `[min, max]`.
* **`lerp(a, b, t)`** - Linear interpolation between `a` and `b`.
* **`fibonacci(n)`** - Nth Fibonacci output.
* **`is_prime(n)`** - Tests primality.
* **`gcd(a, b)`** - Greatest Common Divisor.
* **`factorial(n)`** - Computes `n!`.
* **`TAU`** - Evaluates to `2.0 * PI`.

### `strings`
* **`repeat_str(s, n)`** - Repeats string `s` n-times.
* **`pad_left(s, length, pad_char)`**
* **`pad_right(s, length, pad_char)`**
* **`capitalize(s)`** - Capitalizes the first letter.
* **`title_case(s)`** - Capitalizes the first letter of every word.
* **`is_numeric(s)`** - Checks if a string only contains digits, `.`, or `-`.
* **`char_at(s, index)`**
* **`index_of(s, search)`**

### `collections`
* **`flatten(list)`** - Flattens an array of arrays into a 1D list.
* **`chunk(list, size)`** - Separates arrays into chunks.
* **`unique(list)`** - Deduplicates array elements.
* **`group_by(list, key_fn)`**
* **`count_by(list, key_fn)`**
* **`sum_list(list)`** - Computes the total using `reduce`.
* **`product(list)`**
* **`take(list, n)`** / **`drop(list, n)`**
* **`find(list, predicate)`** - Finds the first element matching `predicate` closure.
* **`find_index(list, predicate)`**
* **`any(list, predicate)`** / **`all(...)`**

### `functional`
* **`compose(f, g)`** - Executes `f(g(arg))`.
* **`pipe(f, g)`** - Executes `g(f(arg))`.
* **`partial(f, bound_arg)`** - Partially applies single argument to `f`.
* **`memoize(f)`** - Caches the results of pure functions given an argument.
* **`identity(x)`**
* **`constant(x)`**

---

## Core Built-ins (No import needed)

These are natively registered in the interpreter environment:

### I/O
* **`print(val)`** - Prints to stdout.
* **`input(prompt?)`** - Reads input line from stdin.
* **`read_file(path)`** - Returns the string contents of a file.
* **`write_file(path, content)`** - Overwrites path with content.
* **`append_file(path, content)`** - Appends content.
* **`file_exists(path)`** - Boolean check.

### Casts & Meta
* **`len(x)`** - Length of string/list/map.
* **`int(x)`**, **`float(x)`**, **`str(x)`**, **`type(x)`**.

### Math
* **`range(start, end)`** - Returns a list of ints.
* **`sqrt`, `abs`, `max`, `min`, `floor`, `ceil`, `round`**, **`pow`, `log`, `sin`, `cos`, `tan`, `atan2`**.

### Collections
* **`push(list, val)`** - Returns new list with pushed value.
* **`pop(list)`** - Returns the last item.
* **`contains(list/str, val)`**
* **`map(list, fn)`**, **`filter(list, fn)`**, **`reduce(list, fn, init)`**
* **`sort(list)`**, **`reverse(list)`**
* **`enumerate(list)`**, **`zip(list1, list2)`** 
* **`keys(map)`**, **`values(map)`**, **`has_key(map, key)`**, **`delete(map, key)`**.

### Strings
* **`split(str, delim)`** - Generates Array of substrings.
* **`join(list, delim)`** - Builds String from an Array of substrings.
* **`replace(str, a, b)`**, **`starts_with`**, **`ends_with`**, **`trim`**.
* **`upper`**, **`lower`**, **`chars`**, **`substr(s, index, len)`**.
