(component
  (type (;0;)
    (component
      (type (;0;) (func (param "left" u32) (param "right" u32) (result u32)))
      (export (;0;) "add" (func (type 0)))
    )
  )
  (component (;0;)
    (core module $numbers (;0;)
      (type (;0;) (func (param i32 i32) (result i32)))
      (func (;0;) (type 0) (param i32 i32) (result i32)
        local.get 0
        local.get 1
        i32.add
      )
      (export "add" (func 0))
    )
    (core instance $firstInstance (;0;) (instantiate $numbers))
    (alias core export $firstInstance "add" (core func (;0;)))
    (type (;0;) (func (param "left" u32) (param "right" u32) (result u32)))
    (func (;0;) (type 0) (canon lift (core func 0)))
    (export (;1;) "add" (func 0))
  )
  (type (;1;)
    (component
      (type (;0;)
        (instance
          (type (;0;) (func (param "left" u32) (param "right" u32) (result u32)))
          (export (;0;) "add" (func (type 0)))
        )
      )
      (import "unlocked-dep=<test:add@{>=1.0.0}>" (instance (;0;) (type 0)))
      (type (;1;) (func (param "input" u32) (result u32)))
      (export (;0;) "first" (func (type 1)))
    )
  )
  (component (;1;)
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
  (type (;2;)
    (component
      (type (;0;)
        (instance
          (type (;0;) (func (param "left" u32) (param "right" u32) (result u32)))
          (export (;0;) "add" (func (type 0)))
        )
      )
      (import "unlocked-dep=<test:add@{>=1.0.0}>" (instance (;0;) (type 0)))
      (type (;1;) (func (param "input" u32) (result u32)))
      (export (;0;) "second" (func (type 1)))
    )
  )
  (component (;2;)
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
        i32.const 5
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
    (export (;2;) "second" (func 1))
  )
  (type (;3;)
    (component
      (type (;0;)
        (instance
          (type (;0;) (func (param "input" u32) (result u32)))
          (export (;0;) "first" (func (type 0)))
        )
      )
      (import "unlocked-dep=<test:inc@{>=1.0.0}>" (instance (;0;) (type 0)))
      (type (;1;)
        (instance
          (type (;0;) (func (param "input" u32) (result u32)))
          (export (;0;) "second" (func (type 0)))
        )
      )
      (import "unlocked-dep=<test:five@{>=1.0.0}>" (instance (;1;) (type 1)))
      (type (;2;) (func (param "left" u32) (param "right" u32) (result u32)))
      (export (;0;) "full" (func (type 2)))
    )
  )
  (component (;3;)
    (type (;0;)
      (instance
        (type (;0;) (func (param "input" u32) (result u32)))
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
    (import "unlocked-dep=<test:five@{>=1.0.0}>" (instance (;1;) (type 1)))
    (alias export 0 "first" (func (;0;)))
    (core func (;0;) (canon lower (func 0)))
    (alias export 1 "second" (func (;1;)))
    (core func (;1;) (canon lower (func 1)))
    (core module $meet (;0;)
      (type (;0;) (func (param i32) (result i32)))
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
    (core instance $total (;2;) (instantiate $meet
        (with "firsty" (instance 0))
        (with "secondy" (instance 1))
      )
    )
    (alias core export $total "full" (core func (;2;)))
    (type (;2;) (func (param "left" u32) (param "right" u32) (result u32)))
    (func (;2;) (type 2) (canon lift (core func 2)))
    (export (;3;) "full" (func 2))
  )
  (instance (;0;) (instantiate 0))
  (instance (;1;) (instantiate 1
      (with "unlocked-dep=<test:add@{>=1.0.0}>" (instance 0))
    )
  )
  (instance (;2;) (instantiate 2
      (with "unlocked-dep=<test:add@{>=1.0.0}>" (instance 0))
    )
  )
  (instance (;3;) (instantiate 3
      (with "unlocked-dep=<test:inc@{>=1.0.0}>" (instance 1))
      (with "unlocked-dep=<test:five@{>=1.0.0}>" (instance 2))
    )
  )
  (alias export 3 "full" (func (;0;)))
  (export (;1;) "full" (func 0))
)