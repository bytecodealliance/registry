// const fetch = require("fetch")
import { hashing } from "./components/hash-checkpoint/hash-checkpoint/hash_checkpoint.js"

const doStuff = async () => {
  let res = await fetch("http://127.0.0.1:8090/v1/fetch/checkpoint")
  console.log({res})
  let body = await res.json()
  console.log({body})
  const { contents, keyId, signature } = body
  console.log({contents, keyId, signature})
  hashing.hashCheckpoint(contents, keyId, signature)
}

doStuff().then(res => console.log({res}))