(component
  (type (;0;)
    (component
      (type (;0;) (func (param "left" u32) (param "right" u32) (result u32)))
      (export (;0;) "add" (func (type 0)))
    )
  )
  (import "locked-dep=<test:add@1.0.0>,integrity=<sha256-d780a28ec35f001cf1e61401ec1ee22dad5821ccf8280affcf135074c7c53cf2>" (component (;0;) (type 0)))
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
  (import "locked-dep=<test:inc@1.0.0>,integrity=<sha256-0cac81441aaee0b7cf4c35855ad58a29b487dfb499a712d5065696d0cc2747a2>" (component (;1;) (type 1)))
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
  (import "locked-dep=<test:five@1.0.0>,integrity=<sha256-0a9255eaa98b0c0724222a4e8f25f9b3e73bd1763cc4d0d0e34303ba2a5a05e9>" (component (;2;) (type 2)))
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
  (import "locked-dep=<test:meet@1.0.0>,integrity=<sha256-7b582e13fd1f798ed86206850112fe01f837fcbf3210ce29eba8eb087e202f62>" (component (;3;) (type 3)))
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