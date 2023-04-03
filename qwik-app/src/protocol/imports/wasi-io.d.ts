export type InputStream = number;
export type OutputStream = number;
export interface StreamError {
}
export namespace WasiIo {
  export function dropInputStream(this: InputStream): void;
  export function write(this: OutputStream, buf: Uint8Array): bigint;
  export function dropOutputStream(this: OutputStream): void;
}
