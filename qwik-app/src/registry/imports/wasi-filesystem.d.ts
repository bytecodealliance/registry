export type Descriptor = number;
export type Filesize = bigint;
export type OutputStream = OutputStream;
/**
 * # Variants
 * 
 * ## `"access"`
 * 
 * ## `"again"`
 * 
 * ## `"already"`
 * 
 * ## `"badf"`
 * 
 * ## `"busy"`
 * 
 * ## `"deadlk"`
 * 
 * ## `"dquot"`
 * 
 * ## `"exist"`
 * 
 * ## `"fbig"`
 * 
 * ## `"ilseq"`
 * 
 * ## `"inprogress"`
 * 
 * ## `"intr"`
 * 
 * ## `"inval"`
 * 
 * ## `"io"`
 * 
 * ## `"isdir"`
 * 
 * ## `"loop"`
 * 
 * ## `"mlink"`
 * 
 * ## `"msgsize"`
 * 
 * ## `"nametoolong"`
 * 
 * ## `"nodev"`
 * 
 * ## `"noent"`
 * 
 * ## `"nolck"`
 * 
 * ## `"nomem"`
 * 
 * ## `"nospc"`
 * 
 * ## `"nosys"`
 * 
 * ## `"notdir"`
 * 
 * ## `"notempty"`
 * 
 * ## `"notrecoverable"`
 * 
 * ## `"notsup"`
 * 
 * ## `"notty"`
 * 
 * ## `"nxio"`
 * 
 * ## `"overflow"`
 * 
 * ## `"perm"`
 * 
 * ## `"pipe"`
 * 
 * ## `"rofs"`
 * 
 * ## `"spipe"`
 * 
 * ## `"txtbsy"`
 * 
 * ## `"xdev"`
 */
export type Errno = 'access' | 'again' | 'already' | 'badf' | 'busy' | 'deadlk' | 'dquot' | 'exist' | 'fbig' | 'ilseq' | 'inprogress' | 'intr' | 'inval' | 'io' | 'isdir' | 'loop' | 'mlink' | 'msgsize' | 'nametoolong' | 'nodev' | 'noent' | 'nolck' | 'nomem' | 'nospc' | 'nosys' | 'notdir' | 'notempty' | 'notrecoverable' | 'notsup' | 'notty' | 'nxio' | 'overflow' | 'perm' | 'pipe' | 'rofs' | 'spipe' | 'txtbsy' | 'xdev';
export interface AtFlags {
  symlinkFollow?: boolean,
}
export type Device = bigint;
export type Inode = bigint;
/**
 * # Variants
 * 
 * ## `"unknown"`
 * 
 * ## `"block-device"`
 * 
 * ## `"character-device"`
 * 
 * ## `"directory"`
 * 
 * ## `"fifo"`
 * 
 * ## `"symbolic-link"`
 * 
 * ## `"regular-file"`
 * 
 * ## `"socket"`
 */
export type DescriptorType = 'unknown' | 'block-device' | 'character-device' | 'directory' | 'fifo' | 'symbolic-link' | 'regular-file' | 'socket';
export type Linkcount = bigint;
export type Datetime = Datetime;
export interface DescriptorStat {
  dev: Device,
  ino: Inode,
  type: DescriptorType,
  nlink: Linkcount,
  size: Filesize,
  atim: Datetime,
  mtim: Datetime,
  ctim: Datetime,
}
export namespace WasiFilesystem {
  export function getPreopens(): [Descriptor, string][];
  export function writeViaStream(this: Descriptor, offset: Filesize): OutputStream;
  export function appendViaStream(this: Descriptor): OutputStream;
  export function createDirectoryAt(this: Descriptor, path: string): void;
  export function statAt(this: Descriptor, atFlags: AtFlags, path: string): DescriptorStat;
  export function dropDescriptor(this: Descriptor): void;
}
