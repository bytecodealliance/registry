import { dropDescriptor as lowering2Callee, getPreopens as lowering5Callee, writeViaStream as lowering6Callee, appendViaStream as lowering7Callee, createDirectoryAt as lowering8Callee, statAt as lowering9Callee } from '@bytecodealliance/preview2-shim/filesystem';
import { exit as lowering3Callee } from '@bytecodealliance/preview2-shim/exit';
import { getEnvironment as lowering11Callee } from '@bytecodealliance/preview2-shim/environment';
import { print as lowering10Callee } from '@bytecodealliance/preview2-shim/stderr';
import { dropInputStream as lowering0Callee, dropOutputStream as lowering1Callee, write as lowering4Callee } from '@bytecodealliance/preview2-shim/io';

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

function toString(val) {
  if (typeof val === 'symbol') throw new TypeError('symbols cannot be converted to strings');
  return String(val);
}

const utf8Decoder = new TextDecoder();

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
let realloc0;

function lowering5(arg0) {
  const ret = lowering5Callee();
  const vec2 = ret;
  const len2 = vec2.length;
  const result2 = realloc0(0, 0, 4, len2 * 12);
  for (let i = 0; i < vec2.length; i++) {
    const e = vec2[i];
    const base = result2 + i * 12;const [tuple0_0, tuple0_1] = e;
    dataView(memory0).setInt32(base + 0, toUint32(tuple0_0), true);
    const ptr1 = utf8Encode(tuple0_1, realloc0, memory0);
    const len1 = utf8EncodedLen;
    dataView(memory0).setInt32(base + 8, len1, true);
    dataView(memory0).setInt32(base + 4, ptr1, true);
  }
  dataView(memory0).setInt32(arg0 + 4, len2, true);
  dataView(memory0).setInt32(arg0 + 0, result2, true);
}

function lowering6(arg0, arg1, arg2) {
  let ret;
  try {
    ret = { tag: 'ok', val: lowering6Callee(arg0 >>> 0, BigInt.asUintN(64, arg1)) };
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

function lowering7(arg0, arg1) {
  let ret;
  try {
    ret = { tag: 'ok', val: lowering7Callee(arg0 >>> 0) };
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

function lowering8(arg0, arg1, arg2, arg3) {
  const ptr0 = arg1;
  const len0 = arg2;
  const result0 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr0, len0));
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
      break;
    }
    case 'err': {
      const e = variant2.val;
      dataView(memory0).setInt8(arg3 + 0, 1, true);
      const val1 = toString(e);
      let enum1;
      switch (val1) {
        case 'access': {
          enum1 = 0;
          break;
        }
        case 'again': {
          enum1 = 1;
          break;
        }
        case 'already': {
          enum1 = 2;
          break;
        }
        case 'badf': {
          enum1 = 3;
          break;
        }
        case 'busy': {
          enum1 = 4;
          break;
        }
        case 'deadlk': {
          enum1 = 5;
          break;
        }
        case 'dquot': {
          enum1 = 6;
          break;
        }
        case 'exist': {
          enum1 = 7;
          break;
        }
        case 'fbig': {
          enum1 = 8;
          break;
        }
        case 'ilseq': {
          enum1 = 9;
          break;
        }
        case 'inprogress': {
          enum1 = 10;
          break;
        }
        case 'intr': {
          enum1 = 11;
          break;
        }
        case 'inval': {
          enum1 = 12;
          break;
        }
        case 'io': {
          enum1 = 13;
          break;
        }
        case 'isdir': {
          enum1 = 14;
          break;
        }
        case 'loop': {
          enum1 = 15;
          break;
        }
        case 'mlink': {
          enum1 = 16;
          break;
        }
        case 'msgsize': {
          enum1 = 17;
          break;
        }
        case 'nametoolong': {
          enum1 = 18;
          break;
        }
        case 'nodev': {
          enum1 = 19;
          break;
        }
        case 'noent': {
          enum1 = 20;
          break;
        }
        case 'nolck': {
          enum1 = 21;
          break;
        }
        case 'nomem': {
          enum1 = 22;
          break;
        }
        case 'nospc': {
          enum1 = 23;
          break;
        }
        case 'nosys': {
          enum1 = 24;
          break;
        }
        case 'notdir': {
          enum1 = 25;
          break;
        }
        case 'notempty': {
          enum1 = 26;
          break;
        }
        case 'notrecoverable': {
          enum1 = 27;
          break;
        }
        case 'notsup': {
          enum1 = 28;
          break;
        }
        case 'notty': {
          enum1 = 29;
          break;
        }
        case 'nxio': {
          enum1 = 30;
          break;
        }
        case 'overflow': {
          enum1 = 31;
          break;
        }
        case 'perm': {
          enum1 = 32;
          break;
        }
        case 'pipe': {
          enum1 = 33;
          break;
        }
        case 'rofs': {
          enum1 = 34;
          break;
        }
        case 'spipe': {
          enum1 = 35;
          break;
        }
        case 'txtbsy': {
          enum1 = 36;
          break;
        }
        case 'xdev': {
          enum1 = 37;
          break;
        }
        default: {
          throw new TypeError(`"${val1}" is not one of the cases of errno`);
        }
      }
      dataView(memory0).setInt8(arg3 + 1, enum1, true);
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
}

function lowering9(arg0, arg1, arg2, arg3, arg4) {
  if ((arg1 & 4294967294) !== 0) {
    throw new TypeError('flags have extraneous bits set');
  }
  const flags0 = {
    symlinkFollow: Boolean(arg1 & 1),
  };
  const ptr1 = arg2;
  const len1 = arg3;
  const result1 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr1, len1));
  let ret;
  try {
    ret = { tag: 'ok', val: lowering9Callee(arg0 >>> 0, flags0, result1) };
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  const variant8 = ret;
  switch (variant8.tag) {
    case 'ok': {
      const e = variant8.val;
      dataView(memory0).setInt8(arg4 + 0, 0, true);
      const {dev: v2_0, ino: v2_1, type: v2_2, nlink: v2_3, size: v2_4, atim: v2_5, mtim: v2_6, ctim: v2_7 } = e;
      dataView(memory0).setBigInt64(arg4 + 8, toUint64(v2_0), true);
      dataView(memory0).setBigInt64(arg4 + 16, toUint64(v2_1), true);
      const val3 = toString(v2_2);
      let enum3;
      switch (val3) {
        case 'unknown': {
          enum3 = 0;
          break;
        }
        case 'block-device': {
          enum3 = 1;
          break;
        }
        case 'character-device': {
          enum3 = 2;
          break;
        }
        case 'directory': {
          enum3 = 3;
          break;
        }
        case 'fifo': {
          enum3 = 4;
          break;
        }
        case 'symbolic-link': {
          enum3 = 5;
          break;
        }
        case 'regular-file': {
          enum3 = 6;
          break;
        }
        case 'socket': {
          enum3 = 7;
          break;
        }
        default: {
          throw new TypeError(`"${val3}" is not one of the cases of descriptor-type`);
        }
      }
      dataView(memory0).setInt8(arg4 + 24, enum3, true);
      dataView(memory0).setBigInt64(arg4 + 32, toUint64(v2_3), true);
      dataView(memory0).setBigInt64(arg4 + 40, toUint64(v2_4), true);
      const {seconds: v4_0, nanoseconds: v4_1 } = v2_5;
      dataView(memory0).setBigInt64(arg4 + 48, toUint64(v4_0), true);
      dataView(memory0).setInt32(arg4 + 56, toUint32(v4_1), true);
      const {seconds: v5_0, nanoseconds: v5_1 } = v2_6;
      dataView(memory0).setBigInt64(arg4 + 64, toUint64(v5_0), true);
      dataView(memory0).setInt32(arg4 + 72, toUint32(v5_1), true);
      const {seconds: v6_0, nanoseconds: v6_1 } = v2_7;
      dataView(memory0).setBigInt64(arg4 + 80, toUint64(v6_0), true);
      dataView(memory0).setInt32(arg4 + 88, toUint32(v6_1), true);
      break;
    }
    case 'err': {
      const e = variant8.val;
      dataView(memory0).setInt8(arg4 + 0, 1, true);
      const val7 = toString(e);
      let enum7;
      switch (val7) {
        case 'access': {
          enum7 = 0;
          break;
        }
        case 'again': {
          enum7 = 1;
          break;
        }
        case 'already': {
          enum7 = 2;
          break;
        }
        case 'badf': {
          enum7 = 3;
          break;
        }
        case 'busy': {
          enum7 = 4;
          break;
        }
        case 'deadlk': {
          enum7 = 5;
          break;
        }
        case 'dquot': {
          enum7 = 6;
          break;
        }
        case 'exist': {
          enum7 = 7;
          break;
        }
        case 'fbig': {
          enum7 = 8;
          break;
        }
        case 'ilseq': {
          enum7 = 9;
          break;
        }
        case 'inprogress': {
          enum7 = 10;
          break;
        }
        case 'intr': {
          enum7 = 11;
          break;
        }
        case 'inval': {
          enum7 = 12;
          break;
        }
        case 'io': {
          enum7 = 13;
          break;
        }
        case 'isdir': {
          enum7 = 14;
          break;
        }
        case 'loop': {
          enum7 = 15;
          break;
        }
        case 'mlink': {
          enum7 = 16;
          break;
        }
        case 'msgsize': {
          enum7 = 17;
          break;
        }
        case 'nametoolong': {
          enum7 = 18;
          break;
        }
        case 'nodev': {
          enum7 = 19;
          break;
        }
        case 'noent': {
          enum7 = 20;
          break;
        }
        case 'nolck': {
          enum7 = 21;
          break;
        }
        case 'nomem': {
          enum7 = 22;
          break;
        }
        case 'nospc': {
          enum7 = 23;
          break;
        }
        case 'nosys': {
          enum7 = 24;
          break;
        }
        case 'notdir': {
          enum7 = 25;
          break;
        }
        case 'notempty': {
          enum7 = 26;
          break;
        }
        case 'notrecoverable': {
          enum7 = 27;
          break;
        }
        case 'notsup': {
          enum7 = 28;
          break;
        }
        case 'notty': {
          enum7 = 29;
          break;
        }
        case 'nxio': {
          enum7 = 30;
          break;
        }
        case 'overflow': {
          enum7 = 31;
          break;
        }
        case 'perm': {
          enum7 = 32;
          break;
        }
        case 'pipe': {
          enum7 = 33;
          break;
        }
        case 'rofs': {
          enum7 = 34;
          break;
        }
        case 'spipe': {
          enum7 = 35;
          break;
        }
        case 'txtbsy': {
          enum7 = 36;
          break;
        }
        case 'xdev': {
          enum7 = 37;
          break;
        }
        default: {
          throw new TypeError(`"${val7}" is not one of the cases of errno`);
        }
      }
      dataView(memory0).setInt8(arg4 + 8, enum7, true);
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
}

function lowering10(arg0, arg1) {
  const ptr0 = arg0;
  const len0 = arg1;
  const result0 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr0, len0));
  lowering10Callee(result0);
}

function lowering11(arg0) {
  const ret = lowering11Callee();
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
let postReturn0;
let realloc1;

function helloWorld() {
  const ret = exports1['hello-world']();
  const ptr0 = dataView(memory0).getInt32(ret + 0, true);
  const len0 = dataView(memory0).getInt32(ret + 4, true);
  const result0 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr0, len0));
  postReturn0(ret);
  return result0;
}

function storeRegistryInfo(arg0) {
  const ptr0 = utf8Encode(arg0, realloc1, memory0);
  const len0 = utf8EncodedLen;
  exports1['store-registry-info'](ptr0, len0);
}

export { helloWorld, storeRegistryInfo }

const $init = (async() => {
  const module0 = fetchCompile(new URL('./client_storage.core.wasm', import.meta.url));
  const module1 = fetchCompile(new URL('./client_storage.core2.wasm', import.meta.url));
  const module2 = base64Compile('AGFzbQEAAAABRgtgBH9/f38AYAF/AGADf35/AGACf38AYAV/f39/fwBgAn9/AGAEf39/fwF/YAN/f38Bf2AFf39/f38Bf2ACf38Bf2ABfwADERAAAQIDAAQFAQYHCAkJCQcKBAUBcAEQEAdSEQEwAAABMQABATIAAgEzAAMBNAAEATUABQE2AAYBNwAHATgACAE5AAkCMTAACgIxMQALAjEyAAwCMTMADQIxNAAOAjE1AA8IJGltcG9ydHMBAArZARAPACAAIAEgAiADQQARAAALCQAgAEEBEQEACw0AIAAgASACQQIRAgALCwAgACABQQMRAwALDwAgACABIAIgA0EEEQAACxEAIAAgASACIAMgBEEFEQQACwsAIAAgAUEGEQUACwkAIABBBxEBAAsPACAAIAEgAiADQQgRBgALDQAgACABIAJBCREHAAsRACAAIAEgAiADIARBChEIAAsLACAAIAFBCxEJAAsLACAAIAFBDBEJAAsLACAAIAFBDREJAAsNACAAIAEgAkEOEQcACwkAIABBDxEKAAsALQlwcm9kdWNlcnMBDHByb2Nlc3NlZC1ieQENd2l0LWNvbXBvbmVudAUwLjcuMQC3BQRuYW1lABMSd2l0LWNvbXBvbmVudDpzaGltAZoFEAAWaW5kaXJlY3Qtd2FzaS1pby13cml0ZQElaW5kaXJlY3Qtd2FzaS1maWxlc3lzdGVtLWdldC1wcmVvcGVucwIpaW5kaXJlY3Qtd2FzaS1maWxlc3lzdGVtLXdyaXRlLXZpYS1zdHJlYW0DKmluZGlyZWN0LXdhc2ktZmlsZXN5c3RlbS1hcHBlbmQtdmlhLXN0cmVhbQQsaW5kaXJlY3Qtd2FzaS1maWxlc3lzdGVtLWNyZWF0ZS1kaXJlY3RvcnktYXQFIGluZGlyZWN0LXdhc2ktZmlsZXN5c3RlbS1zdGF0LWF0BhppbmRpcmVjdC13YXNpLXN0ZGVyci1wcmludAcpaW5kaXJlY3Qtd2FzaS1lbnZpcm9ubWVudC1nZXQtZW52aXJvbm1lbnQIJWFkYXB0LXdhc2lfc25hcHNob3RfcHJldmlldzEtZmRfd3JpdGUJMmFkYXB0LXdhc2lfc25hcHNob3RfcHJldmlldzEtcGF0aF9jcmVhdGVfZGlyZWN0b3J5Ci5hZGFwdC13YXNpX3NuYXBzaG90X3ByZXZpZXcxLXBhdGhfZmlsZXN0YXRfZ2V0CyhhZGFwdC13YXNpX3NuYXBzaG90X3ByZXZpZXcxLWVudmlyb25fZ2V0DC5hZGFwdC13YXNpX3NuYXBzaG90X3ByZXZpZXcxLWVudmlyb25fc2l6ZXNfZ2V0DSthZGFwdC13YXNpX3NuYXBzaG90X3ByZXZpZXcxLWZkX3ByZXN0YXRfZ2V0DjBhZGFwdC13YXNpX3NuYXBzaG90X3ByZXZpZXcxLWZkX3ByZXN0YXRfZGlyX25hbWUPJmFkYXB0LXdhc2lfc25hcHNob3RfcHJldmlldzEtcHJvY19leGl0');
  const module3 = base64Compile('AGFzbQEAAAABRgtgBH9/f38AYAF/AGADf35/AGACf38AYAV/f39/fwBgAn9/AGAEf39/fwF/YAN/f38Bf2AFf39/f38Bf2ACf38Bf2ABfwACZhEAATAAAAABMQABAAEyAAIAATMAAwABNAAAAAE1AAQAATYABQABNwABAAE4AAYAATkABwACMTAACAACMTEACQACMTIACQACMTMACQACMTQABwACMTUACgAIJGltcG9ydHMBcAEQEAkWAQBBAAsQAAECAwQFBgcICQoLDA0ODwAtCXByb2R1Y2VycwEMcHJvY2Vzc2VkLWJ5AQ13aXQtY29tcG9uZW50BTAuNy4xABwEbmFtZQAVFHdpdC1jb21wb25lbnQ6Zml4dXBz');
  Promise.all([module0, module1, module2, module3]).catch(() => {});
  ({ exports: exports0 } = await instantiateCore(await module2));
  ({ exports: exports1 } = await instantiateCore(await module0, {
    wasi_snapshot_preview1: {
      environ_get: exports0['11'],
      environ_sizes_get: exports0['12'],
      fd_prestat_dir_name: exports0['14'],
      fd_prestat_get: exports0['13'],
      fd_write: exports0['8'],
      path_create_directory: exports0['9'],
      path_filestat_get: exports0['10'],
      proc_exit: exports0['15'],
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
      'get-environment': exports0['7'],
    },
    'wasi-exit': {
      exit: lowering3,
    },
    'wasi-filesystem': {
      'append-via-stream': exports0['3'],
      'create-directory-at': exports0['4'],
      'drop-descriptor': lowering2,
      'get-preopens': exports0['1'],
      'stat-at': exports0['5'],
      'write-via-stream': exports0['2'],
    },
    'wasi-io': {
      'drop-input-stream': lowering0,
      'drop-output-stream': lowering1,
      write: exports0['0'],
    },
    'wasi-stderr': {
      print: exports0['6'],
    },
  }));
  memory0 = exports1.memory;
  realloc0 = exports2.cabi_import_realloc;
  ({ exports: exports3 } = await instantiateCore(await module3, {
    '': {
      $imports: exports0.$imports,
      '0': lowering4,
      '1': lowering5,
      '10': exports2.path_filestat_get,
      '11': exports2.environ_get,
      '12': exports2.environ_sizes_get,
      '13': exports2.fd_prestat_get,
      '14': exports2.fd_prestat_dir_name,
      '15': exports2.proc_exit,
      '2': lowering6,
      '3': lowering7,
      '4': lowering8,
      '5': lowering9,
      '6': lowering10,
      '7': lowering11,
      '8': exports2.fd_write,
      '9': exports2.path_create_directory,
    },
  }));
  postReturn0 = exports1['cabi_post_hello-world'];
  realloc1 = exports1.cabi_realloc;
})();

await $init;
