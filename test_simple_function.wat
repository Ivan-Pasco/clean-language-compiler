(module
  ;; WebAssembly version: 1
  ;; Type section
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Type definition
  ;; Import section
  (import "env" "print" ...)
  (import "env" "printl" ...)
  (import "env" "print_simple" ...)
  (import "env" "printl_simple" ...)
  (import "env" "file_write" ...)
  (import "env" "file_read" ...)
  (import "env" "file_exists" ...)
  (import "env" "file_delete" ...)
  (import "env" "file_append" ...)
  (import "env" "http_get" ...)
  (import "env" "http_post" ...)
  (import "env" "http_put" ...)
  (import "env" "http_patch" ...)
  (import "env" "http_delete" ...)
  (import "env" "int_to_string" ...)
  (import "env" "float_to_string" ...)
  (import "env" "bool_to_string" ...)
  (import "env" "string_to_int" ...)
  (import "env" "string_to_float" ...)
  ;; Function section
  (func (type 19))
  (func (type 20))
  (func (type 21))
  (func (type 22))
  (func (type 23))
  (func (type 24))
  (func (type 25))
  (func (type 26))
  (func (type 27))
  (func (type 28))
  (func (type 29))
  (func (type 30))
  (func (type 31))
  (func (type 32))
  (func (type 33))
  (func (type 34))
  (func (type 35))
  (func (type 36))
  (func (type 37))
  (func (type 38))
  (func (type 39))
  (func (type 40))
  (func (type 41))
  (func (type 42))
  (func (type 43))
  (func (type 44))
  (func (type 45))
  (func (type 46))
  (func (type 47))
  (func (type 48))
  (func (type 49))
  (func (type 50))
  (func (type 51))
  (func (type 52))
  (func (type 53))
  (func (type 54))
  (func (type 55))
  (func (type 56))
  (func (type 57))
  (func (type 58))
  (func (type 59))
  (func (type 60))
  (func (type 61))
  (func (type 62))
  (func (type 63))
  (func (type 64))
  (func (type 65))
  (func (type 66))
  (func (type 67))
  (func (type 68))
  (func (type 69))
  (func (type 70))
  (func (type 71))
  (func (type 72))
  (func (type 73))
  (func (type 74))
  (func (type 75))
  (func (type 76))
  (func (type 77))
  (func (type 78))
  (func (type 79))
  (func (type 80))
  ;; Memory section
  (memory 1)
  ;; Export section
  (export "start" (func 80))
  (export "memory" (memory 0))
  (export "string.concat" (func 28))
  (export "string_starts_with" (func 34))
  (export "string_pad_start" (func 69))
  (export "printl_simple" (func 3))
  (export "print_simple" (func 2))
  (export "string_last_index_of" (func 66))
  (export "string_ends_with" (func 35))
  (export "string_ends_with_impl" (func 63))
  (export "string_trim_start_impl" (func 52))
  (export "string.compare" (func 29))
  (export "print" (func 0))
  (export "printl" (func 1))
  (export "matrix.create" (func 73))
  (export "matrix.get" (func 74))
  (export "mustBeTrue" (func 25))
  (export "length" (func 24))
  (export "string_char_at" (func 46))
  (export "string_pad_start_impl" (func 57))
  (export "float_to_string" (func 15))
  (export "string_contains" (func 31))
  (export "mustBeFalse" (func 26))
  (export "string_trim_end_impl" (func 59))
  (export "string_starts_with_impl" (func 62))
  (export "string_to_lower_case_impl" (func 60))
  (export "string_is_blank" (func 49))
  (export "string_trim" (func 70))
  (export "string_trim_impl" (func 58))
  (export "matrix.add" (func 76))
  (export "string_replace" (func 68))
  (export "matrix.transpose" (func 78))
  (export "string_length" (func 30))
  (export "string_to_upper_case" (func 72))
  (export "array_get" (func 19))
  (export "assert" (func 21))
  (export "int_to_string" (func 14))
  (export "string_compare" (func 23))
  (export "string_to_int" (func 17))
  (export "matrix.multiply" (func 77))
  (export "string_pad_end" (func 51))
  (export "string_to_upper" (func 36))
  (export "string_replace_all" (func 45))
  (export "string_replace_impl" (func 56))
  (export "string_substring" (func 67))
  (export "string_index_of" (func 32))
  (export "string_substring_impl" (func 55))
  (export "string_to_float" (func 18))
  (export "string_to_upper_case_impl" (func 61))
  (export "string_concat" (func 22))
  (export "mustBeEqual" (func 27))
  (export "string_trim_start" (func 64))
  (export "string_to_lower_case" (func 71))
  (export "string_is_empty" (func 48))
  (export "string_trim_end" (func 65))
  (export "string_last_index_of_impl" (func 54))
  (export "add" (func 79))
  (export "array_length" (func 20))
  (export "matrix.set" (func 75))
  (export "string_char_code_at" (func 47))
  (export "string_to_lower" (func 37))
  (export "bool_to_string" (func 16))
  ;; Code section start
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
  (func
    ;; Function body
  )
)
