(module
  ;; Export a function named "start" that returns 42
  (func $start (export "start") (result i32)
    i32.const 42  ;; Push 42 onto the stack
    return        ;; Return the value on top of the stack (42)
  )
) 