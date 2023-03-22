enum PERMS {
  RELEASE,
  YANK
}

export class Validator {
  permissions: {key: string, value: PERMS[]}[]
  releases: string[]
  keys: {[k: string]: any}
  algorithm: string | null

  constructor() {
    this.permissions = [];
    this.releases = [];
    this.keys = {};
    this.algorithm = null;
  }

  setAlgo(algo: string) {
    this.algorithm = algo
  }
}