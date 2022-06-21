const fs = require("fs");
const nearAPI = require("near-api-js");
const getConfig = require("./config");
const { nodeUrl, networkId, contractName, contractMethods } =
  getConfig("testnet");

const {
  keyStores: { InMemoryKeyStore },
  Near,
  Account,
  Contract,
  KeyPair,
  utils: {
    format: { parseNearAmount },
  },
} = nearAPI;

let credentials;
console.log("contractName: ", contractName);
try {
  credentials = JSON.parse(
    fs.readFileSync(
      `${process.env.HOME}/.near-credentials/${networkId}/${contractName}.json`
    )
  );
} catch (e) {
  console.warn("credentials not found, looking in /neardev");
  credentials = JSON.parse(
    fs.readFileSync(`./neardev/${networkId}/${contractName}.json`)
  );
}

const keyStore = new InMemoryKeyStore();
keyStore.setKey(
  networkId,
  contractName,
  KeyPair.fromString(credentials.private_key)
);
const near = new Near({
  networkId,
  nodeUrl,
  deps: { keyStore },
});
const { connection } = near;
const contractAccount = new Account(connection, contractName);
contractAccount.addAccessKey = (publicKey) =>
  contractAccount.addKey(
    publicKey,
    contractName,
    contractMethods,
    parseNearAmount("0.1")
  );
const contract = new Contract(contractAccount, contractName, contractMethods);

module.exports = {
  near,
  credentials,
  keyStore,
  connection,
  contract,
  contractName,
  contractAccount,
  contractMethods,
};

// OLD CODE:

// const fs = require("fs");
// const nearAPI = require("near-api-js");
// const getConfig = require("../src/config");
// const { nodeUrl, networkId, contractName, contractMethods } = getConfig();
// const {
//   keyStores: { InMemoryKeyStore },
//   Near,
//   Account,
//   Contract,
//   KeyPair,
//   utils: {
//     format: { parseNearAmount },
//   },
// } = nearAPI;

// const credPath = `./neardev/${contractName}.json`;
// console.log("Loading Credentials:\n", credPath);

// let credentials;
// try {
//   credentials = JSON.parse(fs.readFileSync(credPath));
// } catch (e) {
//   console.warn(e);
//   /// attempt to load backup creds from local machine
//   credentials = JSON.parse(
//     fs.readFileSync(
//       `${process.env.HOME}/.near-credentials/${networkId}/${contractName}.json`
//     )
//   );
// }
// const keyStore = new InMemoryKeyStore();
// keyStore.setKey(
//   networkId,
//   contractName,
//   KeyPair.fromString(credentials.private_key)
// );
// const near = new Near({
//   networkId,
//   nodeUrl,
//   deps: { keyStore },
// });
// const { connection } = near;
// const contractAccount = new Account(connection, contractName);
// contractAccount.addAccessKey = (publicKey) =>
//   contractAccount.addKey(
//     publicKey,
//     contractName,
//     contractMethods.changeMethods,
//     parseNearAmount("0.1")
//   );
// const contract = new Contract(contractAccount, contractName, contractMethods);

// module.exports = {
//   near,
//   keyStore,
//   connection,
//   contract,
//   contractName,
//   contractAccount,
//   contractMethods,
// };
