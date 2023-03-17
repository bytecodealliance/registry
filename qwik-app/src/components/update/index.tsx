import { component$, useStore, NoSerialize, useTask$ } from "@builder.io/qwik"
import { getCheckpoint, hashCheckpoint } from "../../imports"
import { reg } from "../../registry/client_storage"

export default component$((props: {postMessage: NoSerialize<(message: any) => void>, root: string} ) => {
  const store = useStore({content: ""})
  console.log({store})

  useTask$(async ({track}) => {
    track(() => props.root)
    if (props.root) {
      const resp = await fetch("http://127.0.0.1:8090/fetch/logs", {
        headers: { "Content-Type": "application/json" }, method: "POST", body: JSON.stringify({"root": `sha256:${props.root}`, "packages": {"funny": null}})
      })
      const logs = await resp.json()
      console.log({resp}, resp.body, {logs})
    }
  })
  return <>
    Data for updating
    <div>{store.content}</div>
    <div>Root: {props.root}</div>
    <button onClick$={() => {
      store.content = reg.getRegistryPass()
      const checkpoint = getCheckpoint()
      console.log({checkpoint})
      // reg.update()
      // hashCheckpoint(checkpoint)
      console.log({props})
      if (props.postMessage) {
        // props.postMessage("")
        console.log("MAKE REQUEST")
        console.log("data: ", hashCheckpoint(checkpoint))
        props.postMessage({type: "makeRequest", data: hashCheckpoint(checkpoint)})
      }
    }
    }>Update</button>
  </>
})