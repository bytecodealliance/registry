// eslint-disable-next-line @typescript-eslint/no-var-requires

addEventListener("message", async (e) => {
  console.log("EVENT DATA for will", e)
  if (e.data.type === "makeRequest") {
    const xhr = new XMLHttpRequest()
    xhr.open("POST", "/hash/checkpoint", false);
    xhr.send(e.data.data);
    const resp = xhr.responseText
    console.log("WEB WORKER", {resp})
    postMessage(resp)
  } else if (e.data.type === "foo") {
    console.log("IN FOO BRANCH")
    console.log("event", e.data)

  } else if (e.data.type === "opfs") {
    const opfsRoot = await navigator.storage.getDirectory();
    const fileHandle = await opfsRoot.getFileHandle('my first file', {create: true});
    const contents = 'Some text';
    const writable = await fileHandle.createWritable();
    await writable.write(contents);
    await writable.close();

  }
})