/*
 * WHAT IS THIS FILE?
 *
 * The service-worker.ts file is used to have state of the art prefetching.
 * https://qwik.builder.io/qwikcity/prefetching/overview/
 *
 * Qwik uses a service worker to speed up your site and reduce latency, ie, not used in the traditional way of offline.
 * You can also use this file to add more functionality that runs in the service worker.
 */
import { setupServiceWorker } from '@builder.io/qwik-city/service-worker';
import { hashCheckpoint } from "../imports"
setupServiceWorker();

addEventListener('install', () => self.skipWaiting());

addEventListener('activate', () => self.clients.claim());
// const enc = new TextEncoder();

self.addEventListener('fetch', async function(event) {
  console.log("EARLY")
  const body = event.request.body
  console.log({body})
  const reader = body?.getReader()
  let content = new Uint8Array()
  await reader?.read().then((function processText({done, value}) {
    if (done) {
      // content = value
      // console.log({value})
      return value
    }
    // console.log({value})
    content = value
    return reader.read().then(processText)
  }))
  // console.log({content: dec.decode(content)})
  // if (arrayBuffer.length > 0) {
    // let thing = Array.from(body)
    // console.log(thing.map((b) => b.toString(16).padStart(2, "0")).join(""))
  // }
  console.log("INTERCEPTING REQUEST", {event: content})
  if (event.request.url.includes("/hash")) {
    // const content: Uint8Array = event.request.body || new;
    // event.respondWith(crypto.subtle.digest("SHA-256", enc.encode("foo")).then(dig => new Response(dig)))

    event.respondWith(crypto.subtle.digest("SHA-256", content)
      // .then((dig) => console.log({dig}) || Array.from(dig))
      .then(dig => {
        // console.log({dig})
        let temp = Array.from(new Uint8Array(dig))
        let fin = temp.map((b) => b.toString(16).padStart(2, "0")).join("")
        // console.log({fin})
        return fin
      })
      .then(dig => new Response(dig))
    )
  }
  // }
})

// declare const self: ServiceWorkerGlobalScope;
