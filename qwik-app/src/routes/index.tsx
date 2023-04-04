// import type { initialState, NoSerialize } from '@builder.io/qwik';
import { component$, useVisibleTask$, useStore, noSerialize } from '@builder.io/qwik';
import { isBrowser } from '@builder.io/qwik/build';
import type { DocumentHead } from '@builder.io/qwik-city';
// import { reg } from '../registry/client_storage';
import { Checkpoint } from "../components/checkpoint"
// import Update from "../components/update"
import {$init} from "../registry/client_storage.js"
import myWorker from "./web-worker?worker"

export default component$(() => {
  const store = useStore({
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    postMessage: noSerialize(function (message: any) {
      console.log({message})
    }),
    root: ""
  })
  // reg.passthrough();
  useVisibleTask$(async () => {
    if (isBrowser) {
      // const worker = new Worker("web-worker.js", { type: "module" })
      await $init
      const worker = new myWorker()
      worker.addEventListener("message", (event) => {
        console.log("main thread back", {event})
        if (event.data) {
          store.root = event.data
          console.log(store.root, "CAME FROM EVENT")
        }
      })
      store.postMessage = noSerialize(function(message: any) {
        worker.postMessage(message)
      })
      window.localStorage.setItem("url", "http://127.0.0.1:8090")
    }
  })
  console.log("ROOT", store.root)
  return (
    <div>
      <h1>
        <Checkpoint postMessage={store.postMessage} root={store.root}/>
        {/* <Update postMessage={store.postMessage} root={store.root} checkpoint={"foo"}/> */}
      </h1>
    </div>
  );
});

export const head: DocumentHead = {
  title: 'Welcome to Qwik',
  meta: [
    {
      name: 'description',
      content: 'Qwik site description',
    },
  ],
};
