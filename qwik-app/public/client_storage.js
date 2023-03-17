import { dropDescriptor as lowering2Callee, writeViaStream as lowering9Callee, appendViaStream as lowering10Callee } from '@bytecodealliance/preview2-shim/filesystem';
import { exit as lowering3Callee } from '@bytecodealliance/preview2-shim/exit';
import { storeRegistryInfo as lowering4Callee, getRegistryInfo as lowering5Callee, getCheckpoint as lowering6Callee, hashCheckpoint as lowering7Callee } from '../imports';
import { print as lowering11Callee } from '@bytecodealliance/preview2-shim/stderr';
import { getEnvironment as lowering12Callee } from '@bytecodealliance/preview2-shim/environment';
import { dropInputStream as lowering0Callee, dropOutputStream as lowering1Callee, write as lowering8Callee } from '@bytecodealliance/preview2-shim/io';

const instantiateCore = WebAssembly.instantiate;

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

let dv = new DataView(new ArrayBuffer());
const dataView = mem => dv.buffer === mem.buffer ? dv : dv = new DataView(mem.buffer);

function toUint32(val) {
  return val >>> 0;
}

const hasOwnProperty = Object.prototype.hasOwnProperty;

function getErrorPayload(e) {
  if (hasOwnProperty.call(e, 'payload')) return e.payload;
  if (hasOwnProperty.call(e, 'message')) return String(e.message);
  return String(e);
}

const toUint64 = val => BigInt.asUintN(64, val);

function toString(val) {
  if (typeof val === 'symbol') throw new TypeError('symbols cannot be converted to strings');
  return String(val);
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

function lowering4(arg0, arg1) {
  const ptr0 = arg0;
  const len0 = arg1;
  const result0 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr0, len0));
  lowering4Callee(result0);
}
let realloc0;

function lowering5(arg0) {
  const ret = lowering5Callee();
  const ptr0 = utf8Encode(ret, realloc0, memory0);
  const len0 = utf8EncodedLen;
  dataView(memory0).setInt32(arg0 + 4, len0, true);
  dataView(memory0).setInt32(arg0 + 0, ptr0, true);
}

function lowering6(arg0) {
  const ret = lowering6Callee();
  const {contents: v0_0, keyId: v0_1, signature: v0_2 } = ret;
  const {logRoot: v1_0, logLength: v1_1, mapRoot: v1_2 } = v0_0;
  const ptr2 = utf8Encode(v1_0, realloc0, memory0);
  const len2 = utf8EncodedLen;
  dataView(memory0).setInt32(arg0 + 4, len2, true);
  dataView(memory0).setInt32(arg0 + 0, ptr2, true);
  dataView(memory0).setInt32(arg0 + 8, toUint32(v1_1), true);
  const ptr3 = utf8Encode(v1_2, realloc0, memory0);
  const len3 = utf8EncodedLen;
  dataView(memory0).setInt32(arg0 + 16, len3, true);
  dataView(memory0).setInt32(arg0 + 12, ptr3, true);
  const ptr4 = utf8Encode(v0_1, realloc0, memory0);
  const len4 = utf8EncodedLen;
  dataView(memory0).setInt32(arg0 + 24, len4, true);
  dataView(memory0).setInt32(arg0 + 20, ptr4, true);
  const ptr5 = utf8Encode(v0_2, realloc0, memory0);
  const len5 = utf8EncodedLen;
  dataView(memory0).setInt32(arg0 + 32, len5, true);
  dataView(memory0).setInt32(arg0 + 28, ptr5, true);
}

function lowering7(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
  const ptr0 = arg0;
  const len0 = arg1;
  const result0 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr0, len0));
  const ptr1 = arg3;
  const len1 = arg4;
  const result1 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr1, len1));
  const ptr2 = arg5;
  const len2 = arg6;
  const result2 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr2, len2));
  const ptr3 = arg7;
  const len3 = arg8;
  const result3 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr3, len3));
  const ret = lowering7Callee({
    contents: {
      logRoot: result0,
      logLength: arg2 >>> 0,
      mapRoot: result1,
    },
    keyId: result2,
    signature: result3,
  });
  const ptr4 = utf8Encode(ret, realloc0, memory0);
  const len4 = utf8EncodedLen;
  dataView(memory0).setInt32(arg9 + 4, len4, true);
  dataView(memory0).setInt32(arg9 + 0, ptr4, true);
}

function lowering8(arg0, arg1, arg2, arg3) {
  const ptr0 = arg1;
  const len0 = arg2;
  const result0 = new Uint8Array(memory0.buffer.slice(ptr0, ptr0 + len0 * 1));
  let ret;
  try {
    ret = { tag: 'ok', val: lowering8Callee(arg0 >>> 0, result0) };
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

function lowering9(arg0, arg1, arg2) {
  let ret;
  try {
    ret = { tag: 'ok', val: lowering9Callee(arg0 >>> 0, BigInt.asUintN(64, arg1)) };
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

function lowering10(arg0, arg1) {
  let ret;
  try {
    ret = { tag: 'ok', val: lowering10Callee(arg0 >>> 0) };
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

function lowering11(arg0, arg1) {
  const ptr0 = arg0;
  const len0 = arg1;
  const result0 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr0, len0));
  lowering11Callee(result0);
}
let realloc1;

function lowering12(arg0) {
  const ret = lowering12Callee();
  const vec3 = ret;
  const len3 = vec3.length;
  const result3 = realloc1(0, 0, 4, len3 * 16);
  for (let i = 0; i < vec3.length; i++) {
    const e = vec3[i];
    const base = result3 + i * 16;const [tuple0_0, tuple0_1] = e;
    const ptr1 = utf8Encode(tuple0_0, realloc1, memory0);
    const len1 = utf8EncodedLen;
    dataView(memory0).setInt32(base + 4, len1, true);
    dataView(memory0).setInt32(base + 0, ptr1, true);
    const ptr2 = utf8Encode(tuple0_1, realloc1, memory0);
    const len2 = utf8EncodedLen;
    dataView(memory0).setInt32(base + 12, len2, true);
    dataView(memory0).setInt32(base + 8, ptr2, true);
  }
  dataView(memory0).setInt32(arg0 + 4, len3, true);
  dataView(memory0).setInt32(arg0 + 0, result3, true);
}
let exports3;
let postReturn0;
let postReturn1;
let postReturn2;
const reg = {
  helloWorld() {
    if (!_initialized) throwUninitialized();
    const ret = exports1['reg#hello-world']();
    const ptr0 = dataView(memory0).getInt32(ret + 0, true);
    const len0 = dataView(memory0).getInt32(ret + 4, true);
    const result0 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr0, len0));
    postReturn0(ret);
    return result0;
  },
  passthrough() {
    if (!_initialized) throwUninitialized();
    const ret = exports1['reg#passthrough']();
    const ptr0 = dataView(memory0).getInt32(ret + 0, true);
    const len0 = dataView(memory0).getInt32(ret + 4, true);
    const result0 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr0, len0));
    postReturn1(ret);
    return result0;
  },
  getRegistryPass() {
    if (!_initialized) throwUninitialized();
    const ret = exports1['reg#get-registry-pass']();
    const ptr0 = dataView(memory0).getInt32(ret + 0, true);
    const len0 = dataView(memory0).getInt32(ret + 4, true);
    const result0 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr0, len0));
    postReturn2(ret);
    return result0;
  },
  update() {
    if (!_initialized) throwUninitialized();
    exports1['reg#update']();
  },
  
};

export { reg }

let _initialized = false;
export const $init = (async() => {
  const module0 = fetchCompile(new URL('./client_storage.core.wasm', import.meta.url));
  const module1 = fetchCompile(new URL('./client_storage.core2.wasm', import.meta.url));
  const module2 = base64Compile('AGFzbQEAAAABOwlgAn9/AGABfwBgCn9/f39/f39/f38AYAR/f39/AGADf35/AGACf38AYAR/f39/AX9gAn9/AX9gAX8AAw4NAAEBAgMEBQABBgcHCAQFAXABDQ0HQw4BMAAAATEAAQEyAAIBMwADATQABAE1AAUBNgAGATcABwE4AAgBOQAJAjEwAAoCMTEACwIxMgAMCCRpbXBvcnRzAQAKrwENCwAgACABQQARAAALCQAgAEEBEQEACwkAIABBAhEBAAsbACAAIAEgAiADIAQgBSAGIAcgCCAJQQMRAgALDwAgACABIAIgA0EEEQMACw0AIAAgASACQQURBAALCwAgACABQQYRBQALCwAgACABQQcRAAALCQAgAEEIEQEACw8AIAAgASACIANBCREGAAsLACAAIAFBChEHAAsLACAAIAFBCxEHAAsJACAAQQwRCAALAC0JcHJvZHVjZXJzAQxwcm9jZXNzZWQtYnkBDXdpdC1jb21wb25lbnQFMC43LjEAigQEbmFtZQATEndpdC1jb21wb25lbnQ6c2hpbQHtAw0AJGluZGlyZWN0LXN0b3JhZ2Utc3RvcmUtcmVnaXN0cnktaW5mbwEiaW5kaXJlY3Qtc3RvcmFnZS1nZXQtcmVnaXN0cnktaW5mbwIfaW5kaXJlY3Qtc3RvcmFnZS1nZXQtY2hlY2twb2ludAMgaW5kaXJlY3Qtc3RvcmFnZS1oYXNoLWNoZWNrcG9pbnQEFmluZGlyZWN0LXdhc2ktaW8td3JpdGUFKWluZGlyZWN0LXdhc2ktZmlsZXN5c3RlbS13cml0ZS12aWEtc3RyZWFtBippbmRpcmVjdC13YXNpLWZpbGVzeXN0ZW0tYXBwZW5kLXZpYS1zdHJlYW0HGmluZGlyZWN0LXdhc2ktc3RkZXJyLXByaW50CClpbmRpcmVjdC13YXNpLWVudmlyb25tZW50LWdldC1lbnZpcm9ubWVudAklYWRhcHQtd2FzaV9zbmFwc2hvdF9wcmV2aWV3MS1mZF93cml0ZQooYWRhcHQtd2FzaV9zbmFwc2hvdF9wcmV2aWV3MS1lbnZpcm9uX2dldAsuYWRhcHQtd2FzaV9zbmFwc2hvdF9wcmV2aWV3MS1lbnZpcm9uX3NpemVzX2dldAwmYWRhcHQtd2FzaV9zbmFwc2hvdF9wcmV2aWV3MS1wcm9jX2V4aXQ=');
  const module3 = base64Compile('AGFzbQEAAAABOwlgAn9/AGABfwBgCn9/f39/f39/f38AYAR/f39/AGADf35/AGACf38AYAR/f39/AX9gAn9/AX9gAX8AAlQOAAEwAAAAATEAAQABMgABAAEzAAIAATQAAwABNQAEAAE2AAUAATcAAAABOAABAAE5AAYAAjEwAAcAAjExAAcAAjEyAAgACCRpbXBvcnRzAXABDQ0JEwEAQQALDQABAgMEBQYHCAkKCwwALQlwcm9kdWNlcnMBDHByb2Nlc3NlZC1ieQENd2l0LWNvbXBvbmVudAUwLjcuMQAcBG5hbWUAFRR3aXQtY29tcG9uZW50OmZpeHVwcw==');
  Promise.all([module0, module1, module2, module3]).catch(() => {});
  ({ exports: exports0 } = await instantiateCore(await module2));
  ({ exports: exports1 } = await instantiateCore(await module0, {
    storage: {
      'get-checkpoint': exports0['2'],
      'get-registry-info': exports0['1'],
      'hash-checkpoint': exports0['3'],
      'store-registry-info': exports0['0'],
    },
    wasi_snapshot_preview1: {
      environ_get: exports0['10'],
      environ_sizes_get: exports0['11'],
      fd_write: exports0['9'],
      proc_exit: exports0['12'],
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
      'get-environment': exports0['8'],
    },
    'wasi-exit': {
      exit: lowering3,
    },
    'wasi-filesystem': {
      'append-via-stream': exports0['6'],
      'drop-descriptor': lowering2,
      'write-via-stream': exports0['5'],
    },
    'wasi-io': {
      'drop-input-stream': lowering0,
      'drop-output-stream': lowering1,
      write: exports0['4'],
    },
    'wasi-stderr': {
      print: exports0['7'],
    },
  }));
  memory0 = exports1.memory;
  realloc0 = exports1.cabi_realloc;
  realloc1 = exports2.cabi_import_realloc;
  ({ exports: exports3 } = await instantiateCore(await module3, {
    '': {
      $imports: exports0.$imports,
      '0': lowering4,
      '1': lowering5,
      '10': exports2.environ_get,
      '11': exports2.environ_sizes_get,
      '12': exports2.proc_exit,
      '2': lowering6,
      '3': lowering7,
      '4': lowering8,
      '5': lowering9,
      '6': lowering10,
      '7': lowering11,
      '8': lowering12,
      '9': exports2.fd_write,
    },
  }));
  postReturn0 = exports1['cabi_post_reg#hello-world'];
  postReturn1 = exports1['cabi_post_reg#passthrough'];
  postReturn2 = exports1['cabi_post_reg#get-registry-pass'];
  _initialized = true;
})();
