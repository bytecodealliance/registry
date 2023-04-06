import { component$, useStore, NoSerialize, useTask$ } from "@builder.io/qwik"
import { hashCheckpoint } from "../../imports"
import { $init, 
  protocol
 } from "../../protocol/protocol_wasm"
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
      const pkg: ProtoEnvelopeBody[] = logs.packages[first].map(record => {
        const binary_string = window.atob(record.content_bytes)
        const len = binary_string.length;
        const bytes = new Uint8Array(len);
        for (let i = 0; i < len; i++) {
            bytes[i] = binary_string.charCodeAt(i);
        }
        return {
          contentBytes: bytes,
          keyId: record.key_id,
          signature: record.signature
        }
      })
      const info = protocol.validate(pkg)
      console.log({info})
      // const binary_string = window.atob(pkg[0].content_bytes);
      // const len = binary_string.length;
      // const bytes = new Uint8Array(len);
      // for (let i = 0; i < len; i++) {
      //     bytes[i] = binary_string.charCodeAt(i);
      // }

      // console.log({checkpoint: props.checkpoint})
      // const envelope: ProtoEnvelopeBody = {
      //   contentBytes: bytes,
      //   keyId: pkg[0].key_id,
      //   signature: pkg[0].signature
      // }
      // console.log({envelope})
      // const info = protocol.validate(envelope)
      // console.log({info})
      // const abinary_string = window.atob(pkg[1].content_bytes);
      // console.log({abinary_string})
      // const alen = abinary_string.length;
      // const abytes = new Uint8Array(alen);
      // for (let i = 0; i < alen; i++) {
      //     abytes[i] = abinary_string.charCodeAt(i);
      // }
      // console.log({pkg, abytes})
      // const next = protocol.validate({
      //   contentBytes: abytes,
      //   keyId: pkg[1].key_id,
      //   signature: pkg[1].signature
      // })
      // console.log({next})
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