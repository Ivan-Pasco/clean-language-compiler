(module
  (type (;0;) (func (param i32) (result i32)))
  (type (;1;) (func (param i32 i32) (result i32)))
  (type (;2;) (func (param i32) (result i32)))
  (type (;3;) (func (param i32)))
  (type (;4;) (func (param i32 i32) (result i32)))
  (type (;5;) (func (param i32 i32) (result i32)))
  (type (;6;) (func (param i32 i32)))
  (type (;7;) (func (param i32 i32)))
  (type (;8;) (func (param i32)))
  (type (;9;) (func (param i32)))
  (type (;10;) (func (param i32 i32 i32 i32) (result i32)))
  (type (;11;) (func (param i32 i32 i32) (result i32)))
  (type (;12;) (func (param i32 i32) (result i32)))
  (type (;13;) (func (param i32 i32) (result i32)))
  (type (;14;) (func (param i32 i32 i32 i32) (result i32)))
  (type (;15;) (func (param i32 i32 i32) (result i32)))
  (type (;16;) (func (param i32 i32 i32 i32 i32) (result i32)))
  (type (;17;) (func (param i32 i32 i32 i32 i32) (result i32)))
  (type (;18;) (func (param i32 i32 i32 i32 i32) (result i32)))
  (type (;19;) (func (param i32 i32 i32) (result i32)))
  (type (;20;) (func))
  (import "env" "print" (func (;0;) (type 6)))
  (import "env" "printl" (func (;1;) (type 7)))
  (import "env" "print_simple" (func (;2;) (type 8)))
  (import "env" "printl_simple" (func (;3;) (type 9)))
  (import "env" "file_write" (func (;4;) (type 10)))
  (import "env" "file_read" (func (;5;) (type 11)))
  (import "env" "file_exists" (func (;6;) (type 12)))
  (import "env" "file_delete" (func (;7;) (type 13)))
  (import "env" "file_append" (func (;8;) (type 14)))
  (import "env" "http_get" (func (;9;) (type 15)))
  (import "env" "http_post" (func (;10;) (type 16)))
  (import "env" "http_put" (func (;11;) (type 17)))
  (import "env" "http_patch" (func (;12;) (type 18)))
  (import "env" "http_delete" (func (;13;) (type 19)))
  (func (;14;) (type 0) (param i32) (result i32)
    local.get 0
    i32.const 0
    i32.lt_s
    if  ;; label = @1
      i32.const 0
      local.get 0
      i32.sub
      return
    else
      local.get 0
      return
    end
    i32.const 0)
  (func (;15;) (type 1) (param i32 i32) (result i32)
    i32.const 0
    return)
  (func (;16;) (type 2) (param i32) (result i32)
    i32.const 0
    return)
  (func (;17;) (type 3) (param i32)
    local.get 0
    drop)
  (func (;18;) (type 4) (param i32 i32) (result i32)
    local.get 0
    return)
  (func (;19;) (type 5) (param i32 i32) (result i32)
    i32.const 0
    return)
  (func (;20;) (type 20)
    i32.const 42
    call 0
    drop)
  (memory (;0;) 1 16)
  (export "start" (func 20))
  (export "memory" (memory 0))
  (@custom "name" "\15\00\00\00\00\00\00\00\03\00\00\00abs\01\00\00\00\09\00\00\00array_get\02\00\00\00\0c\00\00\00array_length\03\00\00\00\06\00\00\00assert\04\00\00\00\0d\00\00\00string_concat\05\00\00\00\0e\00\00\00string_compare\06\00\00\00\00\00\00\00\07\00\00\00\00\00\00\00\08\00\00\00\00\00\00\00\09\00\00\00\00\00\00\00\0a\00\00\00\00\00\00\00\0b\00\00\00\00\00\00\00\0c\00\00\00\00\00\00\00\0d\00\00\00\00\00\00\00\0e\00\00\00\00\00\00\00\0f\00\00\00\00\00\00\00\10\00\00\00\00\00\00\00\11\00\00\00\00\00\00\00\12\00\00\00\00\00\00\00\13\00\00\00\00\00\00\00\14\00\00\00\05\00\00\00start"))
