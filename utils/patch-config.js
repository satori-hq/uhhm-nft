const fs = require("fs");
const contractName = fs.readFileSync("./neardev/dev-account").toString();
// const path = "./src/config.js";
const path = "./test/config.js";

console.log("contractName in patchConfig: ", contractName);

fs.readFile(path, "utf-8", function (err, data) {
  if (err) throw err;

  data = data.replace(
    /.*const contractName.*/gim,
    `const contractName = '${contractName}';`
  );

  fs.writeFile(path, data, "utf-8", function (err) {
    if (err) throw err;
    console.log("Done!");
  });
});
