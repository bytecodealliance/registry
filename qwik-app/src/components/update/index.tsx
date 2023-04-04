import { component$, useStore, NoSerialize, useTask$ } from "@builder.io/qwik"
import { hashCheckpoint } from "../../imports"
import { reg } from "../../registry/client_storage"
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
      protocol.validate(envelope)
      // const validator = new Validator()
      // if (decoded !== undefined) {
      //   for (const entry of decoded.entries) {
      //     console.log({entry})
      //     if (entry?.init?.hashAlgorithm) {
      //       validator.setAlgo(entry.init.hashAlgorithm)
      //       validator.permissions.push({ key: entry.init.key, value: [PERMS.RELEASE, PERMS.YANK]})
      //       const keyString = entry.init.key.split(":")[1]
      //       const binary_string = window.atob(keyString);
      //       const len = binary_string.length;
      //       const bytes = new Uint8Array(len);
      //       for (let i = 0; i < len; i++) {
      //         bytes[i] = binary_string.charCodeAt(i);
      //       }
      //       const key = await crypto.subtle.importKey("raw", bytes, {name: "ECDSA", namedCurve: "P-256"}, true, ["verify"])
      //       console.log({key})
      //       const keyId = pkg.key_id
      //       validator.keys[keyId] = key
      //       // const sig = pkg.signature.split(":")[1]
      //       const sigBinary = pkg.signature.split(":")[1]
      //       // const sigBinary = window.atob(sig)
      //       // console.log({sigBinary})
      //       const buffer = new ArrayBuffer(sigBinary.length);
      //       const int8View = new Int8Array(buffer);
      //       for (let i = 0, strLen = sigBinary.length; i < strLen; i++) {
      //         int8View[i] = sigBinary.charCodeAt(i);
      //       }
      //       const r = new Int8Array(buffer.slice(4, 36));
      //       const s = new Int8Array(buffer.slice(39));
      //       const sigBytes =  appendBuffer(r, s);
      //       // const sigLen = sigBinary.length
      //       // const sigBytes = new Uint8Array(sigLen)
      //       // for (let i = 0; i < sigLen; i++) {
      //       //   sigBytes[i] = sigBinary.charCodeAt(i);
      //       // }
      //       console.log("SIG ", sigBytes)
      //       // const sigBytes = enc.encode(pkg.signature.split(":")[1])
      //       const prefix = enc.encode("WARG-MAP-CHECKPOINT-SIGNATURE-V0:")
      //       const contentBytesString = pkg.content_bytes
      //       const contentBytesBinary = window.atob(contentBytesString)
      //       const contentBytesLen = contentBytesBinary.length
      //       const contentBytes = new Uint8Array(prefix.byteLength+ contentBytesLen)
      //       contentBytes.set(prefix)
      //       for (let i = 0; i < contentBytesLen; i++) {
      //         contentBytes[prefix.byteLength + i] = contentBytesBinary.charCodeAt(i);
      //       }
      //       console.log({contentBytes})
      //       const foo = crypto.subtle.verify({name: "ECDSA", hash: "SHA-256"}, key, sigBytes, contentBytes)
      //       console.log({foo})
      //     }
      //     console.log({validator})
          
      //   }
      // }

    }
  })
  return <>
    Data for updating
    <div>{store.content}</div>
    <div>Root: {props.root}</div>
    <button onClick$={async () => {
      await $init
      console.log({root: props.root})
      // const resp = await fetch("http://127.0.0.1:8090/fetch/logs", {
      //   headers: { "Content-Type": "application/json" }, method: "POST", body: JSON.stringify({"root": `sha256:${props.root}`, "packages": {"funny": null}})
      // })
      // const logs = await resp.json()
      // console.log({logs})
      store.content = reg.getRegistryPass()
      console.log({checkpoint: props.checkpoint})
      // const { contents } = props.checkpoint 
      // protocol.validate("foo", {
      //   logRoot: {
      //     algo: "sha256",
      //     bytes: enc.encode(contents.log_root)
      //   },
      //   logLength: contents.log_length,
      //   mapRoot: {
      //     algo: "sha256",
      //     bytes: enc.encode(contents.map_root)
      //   }
      // })
      // reg.update()
      // console.log("THE HASH", hashCheckpoint(props.checkpoint))
      // console.log({props})
      if (props.postMessage) {
        // props.postMessage("")
        console.log("MAKE REQUEST")
        // console.log("data: ", hashCheckpoint(checkpoint))
        props.postMessage({type: "makeRequest", data: hashCheckpoint(props.checkpoint)})
      }
    }
    }>Update</button>
  </>
})