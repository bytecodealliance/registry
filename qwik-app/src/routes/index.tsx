import { component$, useVisibleTask$, useStore, noSerialize } from '@builder.io/qwik';
import { isBrowser } from '@builder.io/qwik/build';
import type { DocumentHead } from '@builder.io/qwik-city';
import { Checkpoint } from "../components/checkpoint"
import myWorker from "./web-worker?worker"

export default component$(() => {
  const store = useStore({
    postMessage: noSerialize(function (message: any) {
      console.log({message})
    }),
    root: ""
  })
  useVisibleTask$(async () => {
    if (isBrowser) {
      const worker = new myWorker()
      worker.addEventListener("message", (event) => {
        if (event.data) {
          store.root = event.data
        }
      })
      store.postMessage = noSerialize(function(message: any) {
        worker.postMessage(message)
      })
      window.localStorage.setItem("url", "http://127.0.0.1:8090")
    }
  })
  return (
    <div>
      <h1>
        <Checkpoint postMessage={store.postMessage} root={store.root}/>
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
