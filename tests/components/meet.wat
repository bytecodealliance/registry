(component
  (type (;0;)
      (instance
        (type (;0;) (func  (param "input" u32) (result u32)))
        (export (;0;) "first" (func (type 0)))
      )
  )
  (type (;1;)
    (instance
      (type (;0;) (func (param "input" u32) (result u32)))
      (export (;0;) "second" (func (type 0)))
    )
  )
  (import "unlocked-dep=<test:inc@{>=1.0.0}>" (instance (;0;) (type 0)))
  (import "unlocked-dep=<test:five@{>=1.0.0}>" (instance (;0;) (type 1)))
  (alias export 0 "first" (func (;0;)))
  (core func (;0;) (canon lower (func 0)))
  (alias export 1 "second" (func (;1;)))
  (core func (;1;) (canon lower (func 1)))
  (core module $meet
    (type (;0;) (func (param i32 ) (result i32)))
    (type (;1;) (func (param i32 i32) (result i32)))
    (import "firsty" "first" (func (;0;) (type 0)))
    (import "secondy" "second" (func (;1;) (type 0)))
    (func (;2;) (type 1) (param i32 i32) (result i32)
      local.get 0
      call 0
      local.get 1
      call 1
      i32.add
    )
    (export "full" (func 2))
  )
  (core instance (;0;)
    (export "first" (func 0))
  )
  (core instance (;1;)
    (export "second" (func 1))
  )
  (core instance $total (;1;) (instantiate $meet
      (with "firsty" (instance 0))
      (with "secondy" (instance 1))
    )
  )
  (alias core export $total "full" (core func (;2;)))
  (type (;2;) (func (param "left" u32) (param "right" u32) (result u32)))
  (func (;1;) (type 2) (canon lift (core func 2)))
  (export (;2;) "full" (func 2))
)