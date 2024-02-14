(component
  (core module $numbers
    (func (export "add") (param i32 i32) (result i32)
      local.get 0
      local.get 1
      i32.add
    )
  )
  (core instance $firstInstance (instantiate $numbers))
  (alias core export 0 "add" (core func))
  (type (func (param "left" u32) (param "right" u32) (result u32)))
  (func (type 0) (canon lift (core func 0)))
  (export "add" (func 0))
)