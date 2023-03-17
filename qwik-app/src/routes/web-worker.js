// eslint-disable-next-line @typescript-eslint/no-var-requires
// const reg = require("../src/registry/client_storage.js")
import { reg } from "../registry/client_storage"

addEventListener("message", (e) => {
  console.log("EVENT DATA for will", e)
  if (e.data.type === "makeRequest") {
    const xhr = new XMLHttpRequest()
    xhr.open("POST", "/hash/checkpoint", false);
    xhr.send(e.data.data);
    const resp = xhr.responseText
    console.log("WEB WORKER", {resp})
    postMessage(resp)
  } else if (e.data.type === "foo") {
    console.log({reg})
    console.log("IN FOO BRANCH")
    console.log("event", e.data)

    // reg.passthrough()
  }
})