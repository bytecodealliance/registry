import { dropDescriptor as lowering2Callee, writeViaStream as lowering5Callee, appendViaStream as lowering6Callee } from '@bytecodealliance/preview2-shim/filesystem';
import { exit as lowering3Callee } from '@bytecodealliance/preview2-shim/exit';
import { getEnvironment as lowering9Callee } from '@bytecodealliance/preview2-shim/environment';
import { print as lowering7Callee } from '@bytecodealliance/preview2-shim/stderr';
import { dropInputStream as lowering0Callee, dropOutputStream as lowering1Callee, write as lowering4Callee } from '@bytecodealliance/preview2-shim/io';
import { getRandomBytes as lowering8Callee } from '@bytecodealliance/preview2-shim/random';

const instantiateCore = WebAssembly.instantiate;

const hasOwnProperty = Object.prototype.hasOwnProperty;

function getErrorPayload(e) {
  if (hasOwnProperty.call(e, 'payload')) return e.payload;
  if (hasOwnProperty.call(e, 'message')) return String(e.message);
  return String(e);
}

let dv = new DataView(new ArrayBuffer());
const dataView = mem => dv.buffer === mem.buffer ? dv : dv = new DataView(mem.buffer);

const toUint64 = val => BigInt.asUintN(64, val);

function toUint32(val) {
  return val >>> 0;
}

function toString(val) {
  if (typeof val === 'symbol') throw new TypeError('symbols cannot be converted to strings');
  return String(val);
}

const utf8Decoder = new TextDecoder();

const utf8Encoder = new TextEncoder();

let utf8EncodedLen = 0;
function utf8Encode(s, realloc, memory) {
  if (typeof s !== 'string') throw new TypeError('expected a string');
  if (s.length === 0) {
    utf8EncodedLen = 0;
    return 1;
  }
  let allocLen = 0;
  let ptr = 0;
  let writtenTotal = 0;
  while (s.length > 0) {
    ptr = realloc(ptr, allocLen, 1, allocLen + s.length);
    allocLen += s.length;
    const { read, written } = utf8Encoder.encodeInto(
    s,
    new Uint8Array(memory.buffer, ptr + writtenTotal, allocLen - writtenTotal),
    );
    writtenTotal += written;
    s = s.slice(read);
  }
  if (allocLen > writtenTotal)
  ptr = realloc(ptr, allocLen, 1, writtenTotal);
  utf8EncodedLen = writtenTotal;
  return ptr;
}

function throwUninitialized() {
  throw new TypeError('Wasm uninitialized use `await $init` first');
}

const isNode = typeof process !== 'undefined' && process.versions && process.versions.node;
let _fs;
async function fetchCompile (url) {
  if (isNode) {
    _fs = _fs || await import('fs/promises');
    return WebAssembly.compile(await _fs.readFile(url));
  }
  return fetch(url).then(WebAssembly.compileStreaming);
}

const base64Compile = str => WebAssembly.compile(typeof Buffer !== 'undefined' ? Buffer.from(str, 'base64') : Uint8Array.from(atob(str), b => b.charCodeAt(0)));

let exports0;
let exports1;

function lowering0(arg0) {
  lowering0Callee(arg0 >>> 0);
}

function lowering1(arg0) {
  lowering1Callee(arg0 >>> 0);
}

function lowering2(arg0) {
  lowering2Callee(arg0 >>> 0);
}

function lowering3(arg0) {
  let variant0;
  switch (arg0) {
    case 0: {
      variant0= {
        tag: 'ok',
        val: undefined
      };
      break;
    }
    case 1: {
      variant0= {
        tag: 'err',
        val: undefined
      };
      break;
    }
    default: {
      throw new TypeError('invalid variant discriminant for expected');
    }
  }
  lowering3Callee(variant0);
}
let exports2;
let memory0;

function lowering4(arg0, arg1, arg2, arg3) {
  const ptr0 = arg1;
  const len0 = arg2;
  const result0 = new Uint8Array(memory0.buffer.slice(ptr0, ptr0 + len0 * 1));
  let ret;
  try {
    ret = { tag: 'ok', val: lowering4Callee(arg0 >>> 0, result0) };
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  const variant2 = ret;
  switch (variant2.tag) {
    case 'ok': {
      const e = variant2.val;
      dataView(memory0).setInt8(arg3 + 0, 0, true);
      dataView(memory0).setBigInt64(arg3 + 8, toUint64(e), true);
      break;
    }
    case 'err': {
      const e = variant2.val;
      dataView(memory0).setInt8(arg3 + 0, 1, true);
      const { } = e;
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
}

function lowering5(arg0, arg1, arg2) {
  let ret;
  try {
    ret = { tag: 'ok', val: lowering5Callee(arg0 >>> 0, BigInt.asUintN(64, arg1)) };
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  const variant1 = ret;
  switch (variant1.tag) {
    case 'ok': {
      const e = variant1.val;
      dataView(memory0).setInt8(arg2 + 0, 0, true);
      dataView(memory0).setInt32(arg2 + 4, toUint32(e), true);
      break;
    }
    case 'err': {
      const e = variant1.val;
      dataView(memory0).setInt8(arg2 + 0, 1, true);
      const val0 = toString(e);
      let enum0;
      switch (val0) {
        case 'access': {
          enum0 = 0;
          break;
        }
        case 'again': {
          enum0 = 1;
          break;
        }
        case 'already': {
          enum0 = 2;
          break;
        }
        case 'badf': {
          enum0 = 3;
          break;
        }
        case 'busy': {
          enum0 = 4;
          break;
        }
        case 'deadlk': {
          enum0 = 5;
          break;
        }
        case 'dquot': {
          enum0 = 6;
          break;
        }
        case 'exist': {
          enum0 = 7;
          break;
        }
        case 'fbig': {
          enum0 = 8;
          break;
        }
        case 'ilseq': {
          enum0 = 9;
          break;
        }
        case 'inprogress': {
          enum0 = 10;
          break;
        }
        case 'intr': {
          enum0 = 11;
          break;
        }
        case 'inval': {
          enum0 = 12;
          break;
        }
        case 'io': {
          enum0 = 13;
          break;
        }
        case 'isdir': {
          enum0 = 14;
          break;
        }
        case 'loop': {
          enum0 = 15;
          break;
        }
        case 'mlink': {
          enum0 = 16;
          break;
        }
        case 'msgsize': {
          enum0 = 17;
          break;
        }
        case 'nametoolong': {
          enum0 = 18;
          break;
        }
        case 'nodev': {
          enum0 = 19;
          break;
        }
        case 'noent': {
          enum0 = 20;
          break;
        }
        case 'nolck': {
          enum0 = 21;
          break;
        }
        case 'nomem': {
          enum0 = 22;
          break;
        }
        case 'nospc': {
          enum0 = 23;
          break;
        }
        case 'nosys': {
          enum0 = 24;
          break;
        }
        case 'notdir': {
          enum0 = 25;
          break;
        }
        case 'notempty': {
          enum0 = 26;
          break;
        }
        case 'notrecoverable': {
          enum0 = 27;
          break;
        }
        case 'notsup': {
          enum0 = 28;
          break;
        }
        case 'notty': {
          enum0 = 29;
          break;
        }
        case 'nxio': {
          enum0 = 30;
          break;
        }
        case 'overflow': {
          enum0 = 31;
          break;
        }
        case 'perm': {
          enum0 = 32;
          break;
        }
        case 'pipe': {
          enum0 = 33;
          break;
        }
        case 'rofs': {
          enum0 = 34;
          break;
        }
        case 'spipe': {
          enum0 = 35;
          break;
        }
        case 'txtbsy': {
          enum0 = 36;
          break;
        }
        case 'xdev': {
          enum0 = 37;
          break;
        }
        default: {
          throw new TypeError(`"${val0}" is not one of the cases of errno`);
        }
      }
      dataView(memory0).setInt8(arg2 + 4, enum0, true);
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
}

function lowering6(arg0, arg1) {
  let ret;
  try {
    ret = { tag: 'ok', val: lowering6Callee(arg0 >>> 0) };
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  const variant1 = ret;
  switch (variant1.tag) {
    case 'ok': {
      const e = variant1.val;
      dataView(memory0).setInt8(arg1 + 0, 0, true);
      dataView(memory0).setInt32(arg1 + 4, toUint32(e), true);
      break;
    }
    case 'err': {
      const e = variant1.val;
      dataView(memory0).setInt8(arg1 + 0, 1, true);
      const val0 = toString(e);
      let enum0;
      switch (val0) {
        case 'access': {
          enum0 = 0;
          break;
        }
        case 'again': {
          enum0 = 1;
          break;
        }
        case 'already': {
          enum0 = 2;
          break;
        }
        case 'badf': {
          enum0 = 3;
          break;
        }
        case 'busy': {
          enum0 = 4;
          break;
        }
        case 'deadlk': {
          enum0 = 5;
          break;
        }
        case 'dquot': {
          enum0 = 6;
          break;
        }
        case 'exist': {
          enum0 = 7;
          break;
        }
        case 'fbig': {
          enum0 = 8;
          break;
        }
        case 'ilseq': {
          enum0 = 9;
          break;
        }
        case 'inprogress': {
          enum0 = 10;
          break;
        }
        case 'intr': {
          enum0 = 11;
          break;
        }
        case 'inval': {
          enum0 = 12;
          break;
        }
        case 'io': {
          enum0 = 13;
          break;
        }
        case 'isdir': {
          enum0 = 14;
          break;
        }
        case 'loop': {
          enum0 = 15;
          break;
        }
        case 'mlink': {
          enum0 = 16;
          break;
        }
        case 'msgsize': {
          enum0 = 17;
          break;
        }
        case 'nametoolong': {
          enum0 = 18;
          break;
        }
        case 'nodev': {
          enum0 = 19;
          break;
        }
        case 'noent': {
          enum0 = 20;
          break;
        }
        case 'nolck': {
          enum0 = 21;
          break;
        }
        case 'nomem': {
          enum0 = 22;
          break;
        }
        case 'nospc': {
          enum0 = 23;
          break;
        }
        case 'nosys': {
          enum0 = 24;
          break;
        }
        case 'notdir': {
          enum0 = 25;
          break;
        }
        case 'notempty': {
          enum0 = 26;
          break;
        }
        case 'notrecoverable': {
          enum0 = 27;
          break;
        }
        case 'notsup': {
          enum0 = 28;
          break;
        }
        case 'notty': {
          enum0 = 29;
          break;
        }
        case 'nxio': {
          enum0 = 30;
          break;
        }
        case 'overflow': {
          enum0 = 31;
          break;
        }
        case 'perm': {
          enum0 = 32;
          break;
        }
        case 'pipe': {
          enum0 = 33;
          break;
        }
        case 'rofs': {
          enum0 = 34;
          break;
        }
        case 'spipe': {
          enum0 = 35;
          break;
        }
        case 'txtbsy': {
          enum0 = 36;
          break;
        }
        case 'xdev': {
          enum0 = 37;
          break;
        }
        default: {
          throw new TypeError(`"${val0}" is not one of the cases of errno`);
        }
      }
      dataView(memory0).setInt8(arg1 + 4, enum0, true);
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
}

function lowering7(arg0, arg1) {
  const ptr0 = arg0;
  const len0 = arg1;
  const result0 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr0, len0));
  lowering7Callee(result0);
}
let realloc0;

function lowering8(arg0, arg1) {
  const ret = lowering8Callee(arg0 >>> 0);
  const val0 = ret;
  const len0 = val0.byteLength;
  const ptr0 = realloc0(0, 0, 1, len0 * 1);
  const src0 = new Uint8Array(val0.buffer || val0, val0.byteOffset, len0 * 1);
  (new Uint8Array(memory0.buffer, ptr0, len0 * 1)).set(src0);
  dataView(memory0).setInt32(arg1 + 4, len0, true);
  dataView(memory0).setInt32(arg1 + 0, ptr0, true);
}

function lowering9(arg0) {
  const ret = lowering9Callee();
  const vec3 = ret;
  const len3 = vec3.length;
  const result3 = realloc0(0, 0, 4, len3 * 16);
  for (let i = 0; i < vec3.length; i++) {
    const e = vec3[i];
    const base = result3 + i * 16;const [tuple0_0, tuple0_1] = e;
    const ptr1 = utf8Encode(tuple0_0, realloc0, memory0);
    const len1 = utf8EncodedLen;
    dataView(memory0).setInt32(base + 4, len1, true);
    dataView(memory0).setInt32(base + 0, ptr1, true);
    const ptr2 = utf8Encode(tuple0_1, realloc0, memory0);
    const len2 = utf8EncodedLen;
    dataView(memory0).setInt32(base + 12, len2, true);
    dataView(memory0).setInt32(base + 8, ptr2, true);
  }
  dataView(memory0).setInt32(arg0 + 4, len3, true);
  dataView(memory0).setInt32(arg0 + 0, result3, true);
}
let exports3;
let realloc1;
let postReturn0;
let postReturn1;
const protocol = {
  validate(arg0) {
    if (!_initialized) throwUninitialized();
    const {contentBytes: v0_0, keyId: v0_1, signature: v0_2 } = arg0;
    const val1 = v0_0;
    const len1 = val1.byteLength;
    const ptr1 = realloc1(0, 0, 1, len1 * 1);
    const src1 = new Uint8Array(val1.buffer || val1, val1.byteOffset, len1 * 1);
    (new Uint8Array(memory0.buffer, ptr1, len1 * 1)).set(src1);
    const ptr2 = utf8Encode(v0_1, realloc1, memory0);
    const len2 = utf8EncodedLen;
    const ptr3 = utf8Encode(v0_2, realloc1, memory0);
    const len3 = utf8EncodedLen;
    const ret = exports1['protocol#validate'](ptr1, len1, ptr2, 71, ptr3, len3);
    const ptr4 = dataView(memory0).getInt32(ret + 0, true);
    const len4 = dataView(memory0).getInt32(ret + 4, true);
    const result4 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr4, len4));
    let variant6;
    switch (dataView(memory0).getUint8(ret + 8, true)) {
      case 0: {
        variant6 = null;
        break;
      }
      case 1: {
        const ptr5 = dataView(memory0).getInt32(ret + 12, true);
        const len5 = dataView(memory0).getInt32(ret + 16, true);
        const result5 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr5, len5));
        variant6 = result5;
        break;
      }
      default: {
        throw new TypeError('invalid variant discriminant for option');
      }
    }
    let variant8;
    switch (dataView(memory0).getUint8(ret + 20, true)) {
      case 0: {
        variant8 = null;
        break;
      }
      case 1: {
        let enum7;
        switch (dataView(memory0).getUint8(ret + 21, true)) {
          case 0: {
            enum7 = 'sha256';
            break;
          }
          default: {
            throw new TypeError('invalid discriminant specified for HashAlgorithm');
          }
        }
        variant8 = enum7;
        break;
      }
      default: {
        throw new TypeError('invalid variant discriminant for option');
      }
    }
    let variant15;
    switch (dataView(memory0).getUint8(ret + 24, true)) {
      case 0: {
        variant15 = null;
        break;
      }
      case 1: {
        let variant12;
        switch (dataView(memory0).getUint8(ret + 28, true)) {
          case 0: {
            let enum9;
            switch (dataView(memory0).getUint8(ret + 32, true)) {
              case 0: {
                enum9 = 'sha256';
                break;
              }
              default: {
                throw new TypeError('invalid discriminant specified for HashAlgorithm');
              }
            }
            let variant11;
            switch (dataView(memory0).getUint8(ret + 36, true)) {
              case 0: {
                variant11 = null;
                break;
              }
              case 1: {
                const ptr10 = dataView(memory0).getInt32(ret + 40, true);
                const len10 = dataView(memory0).getInt32(ret + 44, true);
                const result10 = new Uint8Array(memory0.buffer.slice(ptr10, ptr10 + len10 * 1));
                variant11 = result10;
                break;
              }
              default: {
                throw new TypeError('invalid variant discriminant for option');
              }
            }
            variant12= {
              tag: 'dyn-hash',
              val: {
                algo: enum9,
                bytes: variant11,
              }
            };
            break;
          }
          default: {
            throw new TypeError('invalid variant discriminant for RecordId');
          }
        }
        let variant14;
        switch (dataView(memory0).getUint8(ret + 48, true)) {
          case 0: {
            variant14 = null;
            break;
          }
          case 1: {
            const ptr13 = dataView(memory0).getInt32(ret + 52, true);
            const len13 = dataView(memory0).getInt32(ret + 56, true);
            const result13 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr13, len13));
            variant14 = result13;
            break;
          }
          default: {
            throw new TypeError('invalid variant discriminant for option');
          }
        }
        variant15 = {
          digest: variant12,
          timestamp: variant14,
        };
        break;
      }
      default: {
        throw new TypeError('invalid variant discriminant for option');
      }
    }
    const len19 = dataView(memory0).getInt32(ret + 64, true);
    const base19 = dataView(memory0).getInt32(ret + 60, true);
    const result19 = [];
    for (let i = 0; i < len19; i++) {
      const base = base19 + i * 16;
      const ptr16 = dataView(memory0).getInt32(base + 0, true);
      const len16 = dataView(memory0).getInt32(base + 4, true);
      const result16 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr16, len16));
      const len18 = dataView(memory0).getInt32(base + 12, true);
      const base18 = dataView(memory0).getInt32(base + 8, true);
      const result18 = [];
      for (let i = 0; i < len18; i++) {
        const base = base18 + i * 1;
        let enum17;
        switch (dataView(memory0).getUint8(base + 0, true)) {
          case 0: {
            enum17 = 'release';
            break;
          }
          case 1: {
            enum17 = 'yank';
            break;
          }
          default: {
            throw new TypeError('invalid discriminant specified for Perm');
          }
        }
        result18.push(enum17);
      }
      result19.push({
        keyId: result16,
        perms: result18,
      });
    }
    let variant23;
    switch (dataView(memory0).getUint8(ret + 68, true)) {
      case 0: {
        variant23 = null;
        break;
      }
      case 1: {
        const len22 = dataView(memory0).getInt32(ret + 76, true);
        const base22 = dataView(memory0).getInt32(ret + 72, true);
        const result22 = [];
        for (let i = 0; i < len22; i++) {
          const base = base22 + i * 16;
          const ptr20 = dataView(memory0).getInt32(base + 0, true);
          const len20 = dataView(memory0).getInt32(base + 4, true);
          const result20 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr20, len20));
          const ptr21 = dataView(memory0).getInt32(base + 8, true);
          const len21 = dataView(memory0).getInt32(base + 12, true);
          const result21 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr21, len21));
          result22.push({
            keyId: result20,
            publicKey: result21,
          });
        }
        variant23 = result22;
        break;
      }
      default: {
        throw new TypeError('invalid variant discriminant for option');
      }
    }
    postReturn0(ret);
    return {
      name: result4,
      checkpoint: variant6,
      state: {
        algorithm: variant8,
        head: variant15,
        permissions: result19,
        keys: variant23,
      },
    };
  },
  
};

function helloWorld() {
  if (!_initialized) throwUninitialized();
  const ret = exports1['hello-world']();
  const ptr0 = dataView(memory0).getInt32(ret + 0, true);
  const len0 = dataView(memory0).getInt32(ret + 4, true);
  const result0 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr0, len0));
  postReturn1(ret);
  return result0;
}

export { protocol, helloWorld }

let _initialized = false;
export const $init = (async() => {
  const module0 = fetchCompile(new URL('./proto_comp.core.wasm', import.meta.url));
  const module1 = fetchCompile(new URL('./proto_comp.core2.wasm', import.meta.url));
  const module2 = base64Compile('AGFzbQEAAAABLghgBH9/f38AYAN/fn8AYAJ/fwBgAn9/AGABfwBgBH9/f38Bf2ACf38Bf2ABfwADDAsAAQIDAgQFBgYGBwQFAXABCwsHOQwBMAAAATEAAQEyAAIBMwADATQABAE1AAUBNgAGATcABwE4AAgBOQAJAjEwAAoIJGltcG9ydHMBAAqLAQsPACAAIAEgAiADQQARAAALDQAgACABIAJBAREBAAsLACAAIAFBAhECAAsLACAAIAFBAxEDAAsLACAAIAFBBBECAAsJACAAQQURBAALDwAgACABIAIgA0EGEQUACwsAIAAgAUEHEQYACwsAIAAgAUEIEQYACwsAIAAgAUEJEQYACwkAIABBChEHAAsALQlwcm9kdWNlcnMBDHByb2Nlc3NlZC1ieQENd2l0LWNvbXBvbmVudAUwLjcuMQDNAwRuYW1lABMSd2l0LWNvbXBvbmVudDpzaGltAbADCwAWaW5kaXJlY3Qtd2FzaS1pby13cml0ZQEpaW5kaXJlY3Qtd2FzaS1maWxlc3lzdGVtLXdyaXRlLXZpYS1zdHJlYW0CKmluZGlyZWN0LXdhc2ktZmlsZXN5c3RlbS1hcHBlbmQtdmlhLXN0cmVhbQMaaW5kaXJlY3Qtd2FzaS1zdGRlcnItcHJpbnQEJWluZGlyZWN0LXdhc2ktcmFuZG9tLWdldC1yYW5kb20tYnl0ZXMFKWluZGlyZWN0LXdhc2ktZW52aXJvbm1lbnQtZ2V0LWVudmlyb25tZW50BiVhZGFwdC13YXNpX3NuYXBzaG90X3ByZXZpZXcxLWZkX3dyaXRlBydhZGFwdC13YXNpX3NuYXBzaG90X3ByZXZpZXcxLXJhbmRvbV9nZXQIKGFkYXB0LXdhc2lfc25hcHNob3RfcHJldmlldzEtZW52aXJvbl9nZXQJLmFkYXB0LXdhc2lfc25hcHNob3RfcHJldmlldzEtZW52aXJvbl9zaXplc19nZXQKJmFkYXB0LXdhc2lfc25hcHNob3RfcHJldmlldzEtcHJvY19leGl0');
  const module3 = base64Compile('AGFzbQEAAAABLghgBH9/f38AYAN/fn8AYAJ/fwBgAn9/AGABfwBgBH9/f38Bf2ACf38Bf2ABfwACSAwAATAAAAABMQABAAEyAAIAATMAAwABNAACAAE1AAQAATYABQABNwAGAAE4AAYAATkABgACMTAABwAIJGltcG9ydHMBcAELCwkRAQBBAAsLAAECAwQFBgcICQoALQlwcm9kdWNlcnMBDHByb2Nlc3NlZC1ieQENd2l0LWNvbXBvbmVudAUwLjcuMQAcBG5hbWUAFRR3aXQtY29tcG9uZW50OmZpeHVwcw==');
  Promise.all([module0, module1, module2, module3]).catch(() => {});
  ({ exports: exports0 } = await instantiateCore(await module2));
  ({ exports: exports1 } = await instantiateCore(await module0, {
    wasi_snapshot_preview1: {
      environ_get: exports0['8'],
      environ_sizes_get: exports0['9'],
      fd_write: exports0['6'],
      proc_exit: exports0['10'],
      random_get: exports0['7'],
    },
  }));
  ({ exports: exports2 } = await instantiateCore(await module1, {
    __main_module__: {
      cabi_realloc: exports1.cabi_realloc,
    },
    env: {
      memory: exports1.memory,
    },
    'wasi-environment': {
      'get-environment': exports0['5'],
    },
    'wasi-exit': {
      exit: lowering3,
    },
    'wasi-filesystem': {
      'append-via-stream': exports0['2'],
      'drop-descriptor': lowering2,
      'write-via-stream': exports0['1'],
    },
    'wasi-io': {
      'drop-input-stream': lowering0,
      'drop-output-stream': lowering1,
      write: exports0['0'],
    },
    'wasi-random': {
      'get-random-bytes': exports0['4'],
    },
    'wasi-stderr': {
      print: exports0['3'],
    },
  }));
  memory0 = exports1.memory;
  realloc0 = exports2.cabi_import_realloc;
  ({ exports: exports3 } = await instantiateCore(await module3, {
    '': {
      $imports: exports0.$imports,
      '0': lowering4,
      '1': lowering5,
      '10': exports2.proc_exit,
      '2': lowering6,
      '3': lowering7,
      '4': lowering8,
      '5': lowering9,
      '6': exports2.fd_write,
      '7': exports2.random_get,
      '8': exports2.environ_get,
      '9': exports2.environ_sizes_get,
    },
  }));
  realloc1 = exports1.cabi_realloc;
  postReturn0 = exports1['cabi_post_protocol#validate'];
  postReturn1 = exports1['cabi_post_hello-world'];
  _initialized = true;
})();
