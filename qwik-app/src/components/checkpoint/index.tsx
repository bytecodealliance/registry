import { component$, useVisibleTask$, useStore, NoSerialize } from "@builder.io/qwik"
import Update from "../update/index"
export const Checkpoint = component$((props: {postMessage: NoSerialize<(message: any) => void>, root: string}) => {
  const store = useStore({ checkpoint: {key_id: ""}})

  useVisibleTask$(async () => {
    if (props.postMessage) {
      console.log("POSTING FOO MESSAGE")
      props.postMessage({type: "foo"})
    }
    const resp = await fetch("http://127.0.0.1:8090/fetch/checkpoint")
      const waited = await resp.json()
      console.log({waited})
      store.checkpoint = waited.checkpoint;
      console.log("IN TASK", {props: props})
      console.log({store})
    })
  console.log({props: props.root})
  // console.log({store})
  return <div>
    <Update postMessage={props.postMessage} root={props.root} 
    checkpoint={store.checkpoint}
    />
    {store.checkpoint.key_id}
    </div>
})