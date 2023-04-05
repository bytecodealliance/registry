import { component$, useStore, NoSerialize, useTask$ } from "@builder.io/qwik"
import { hashCheckpoint } from "../../imports"
// import { reg } from "../../registry/client_storage"
import { $init, 
  protocol
 } from "../../protocol/proto_comp"
import { ProtoEnvelopeBody } from "~/protocol/exports/protocol"

export default component$((props: {
  postMessage: NoSerialize<(message: any) => void>,
  root: string,
  checkpoint: any} ) => {
  const store = useStore({content: ""})
  console.log({store})

  useTask$(async ({track}) => {
    track(() => props.root)
    if (props.root) {
      const resp = await fetch("http://127.0.0.1:8090/fetch/logs", {
        headers: { "Content-Type": "application/json" }, method: "POST", body: JSON.stringify({"root": `sha256:${props.root}`, "packages": {"funny": null}})
      })
      const logs: {packages: Record<string,
        {content_bytes: string, key_id: string, signature: string}[]>
      } = await resp.json()
      console.log({logs})
      const first = Object.keys(logs.packages)[0]
      const pkg: {content_bytes: string, key_id: string, signature: string}[] = logs.packages[first]
      const binary_string = window.atob(pkg[0].content_bytes);
      const len = binary_string.length;
      const bytes = new Uint8Array(len);
      for (let i = 0; i < len; i++) {
          bytes[i] = binary_string.charCodeAt(i);
      }
      console.log({checkpoint: props.checkpoint})
      const envelope: ProtoEnvelopeBody = {
        // contentBytes: enc.encode(pkg[0].content_bytes),
        contentBytes: bytes,
        keyId: pkg[0].key_id,
        signature: pkg[0].signature
      }
      console.log({envelope})
      const info = protocol.validate(envelope)
      console.log({info})
    }
  })
  return <>
    Data for updating
    <div>{store.content}</div>
    <div>Root: {props.root}</div>
    <button onClick$={async () => {
      await $init
      console.log({root: props.root})
      console.log({checkpoint: props.checkpoint})
      if (props.postMessage) {
        // props.postMessage("")
        console.log("MAKE REQUEST")
        props.postMessage({type: "opfs"})
        // console.log("data: ", hashCheckpoint(checkpoint))
        props.postMessage({type: "makeRequest", data: hashCheckpoint(props.checkpoint)})
      }
    }
    }>Update</button>
  </>
})