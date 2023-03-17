import { component$, useVisibleTask$, useStore, NoSerialize } from "@builder.io/qwik"
export const Checkpoint = component$((props: {postMessage: NoSerialize<(message: any) => void>}) => {
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
    })
  console.log({store})
  return <div>{store.checkpoint.key_id}</div>
})