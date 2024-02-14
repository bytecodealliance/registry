(component
  (type (;0;)
    (instance
      (type (;0;) (func (param "left" u32) (param "right" u32) (result u32)))
      (export (;0;) "add" (func (type 0)))
    )
  )
  (import "unlocked-dep=<test:add@{>=1.0.0}>" (instance (;0;) (type 0)))
  (alias export 0 "add" (func (;0;)))
  (core func (;0;) (canon lower (func 0)))
  (core module $numbers (;0;)
    (type (;0;) (func (param i32 i32) (result i32)))
    (type (;1;) (func (param i32) (result i32)))
    (import "adder" "add" (func (;0;) (type 0)))
    (func (;1;) (type 1) (param i32) (result i32)
      local.get 0
      i32.const 2
      call 0
    )
    (export "both" (func 1))
  )
  (core instance (;0;)
    (export "add" (func 0))
  )
  (core instance $firstInstance (;1;) (instantiate $numbers
      (with "adder" (instance 0))
    )
  )
  (alias core export $firstInstance "both" (core func (;1;)))
  (type (;1;) (func (param "input" u32) (result u32)))
  (func (;1;) (type 1) (canon lift (core func 1)))
  (export (;2;) "first" (func 1))
)