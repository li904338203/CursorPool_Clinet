declare module 'sm-crypto' {
  export const sm2: {
    doEncrypt: (msg: string, publicKey: string, cipherMode?: number) => string;
    doDecrypt: (encryptData: string, privateKey: string, cipherMode?: number) => string;
    generateKeyPairHex: () => { privateKey: string; publicKey: string };
  };
  
  export const sm3: (data: string) => string;
  
  export const sm4: {
    encrypt: (plainText: string, key: string) => string;
    decrypt: (cipherText: string, key: string) => string;
  };
} 