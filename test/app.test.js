const fs = require("fs");
const assert = require("assert");
const testUtils = require("./test-utils");
const nearAPI = require("near-api-js");
const BN = require("bn.js");
const {
  utils: {
    format: { parseNearAmount, formatNearAmount },
  },
  transactions: { deployContract, functionCall },
} = nearAPI;

const {
  gas,
  contractId,
  contractAccount,
  getAccount,
  createOrInitAccount,
  getAccountBalance,
} = testUtils;

const APPROVALS_TO_ATTEMPT = 2;
const TOKEN_DELIMETER = ":";
const BOB_ROYALTY = 1000;

describe("NFT Series", function () {
  this.timeout(120000);

  const now = Date.now().toString();

  const tokenType = "test";

  const token_id = tokenType + TOKEN_DELIMETER + Date.now();

  const metadata = {
    media: "https://media.giphy.com/media/h2ZVjT3kt193cxnwm1/giphy.gif",
    issued_at: now.toString(),
  };

  /// users
  const aliceId = "alice-" + now + "." + contractId;
  const bobId = "bob-" + now + "." + contractId;
  const marketId = "market." + contractId;

  const ownerId = "owner." + contractId;

  let alice, bob, market, owner;

  it("should create user & contract accounts", async function () {
    alice = await getAccount(aliceId);
    bob = await getAccount(bobId);
    console.log("\n\n created:", aliceId, "\n\n");

    owner = await createOrInitAccount(ownerId);

    market = await createOrInitAccount(marketId);
    const marketState = await market.state();
    if (marketState.code_hash === "11111111111111111111111111111111") {
      const marketBytes = fs.readFileSync("./out/market.wasm");
      console.log(
        "\n\n deploying market contractBytes:",
        marketBytes.length,
        "\n\n"
      );
      const newMarketArgs = {
        owner_id: contractId,
      };
      const actions = [
        deployContract(marketBytes),
        functionCall("new", newMarketArgs, gas),
      ];
      await market.signAndSendTransaction(marketId, actions);
      console.log("\n\n created:", marketId, "\n\n");
    }
  });

  it("should be deployed", async function () {
    const state = await contractAccount.state();
    try {
      await contractAccount.functionCall({
        contractId,
        methodName: "new_default_meta",
        args: {
          owner_id: contractId,
          supply_cap_by_type: {
            test: "1000000",
          },
        },
        gas,
      });
    } catch (e) {
      if (!/contract has already been initialized/.test(e.toString())) {
        console.warn(e);
      }
    }

    assert.notStrictEqual(state.code_hash, "11111111111111111111111111111111");
  });

  it("should allow the owner to update the contract's base_uri", async function () {
    const updatedBaseUri = "https://ipfs.io";

    await contractAccount.functionCall({
      contractId,
      methodName: "patch_base_uri",
      args: {
        base_uri: updatedBaseUri,
      },
      gas,
      attachedDeposit: parseNearAmount("0.1"),
    });

    const metadata_updated = await contractAccount.viewFunction(
      contractId,
      "nft_metadata"
    );

    assert.strictEqual(metadata_updated.base_uri, updatedBaseUri);
  });

  it("should allow the owner to update all fields of a contract's source_metadata", async function () {
    const updatedVersion = Date.now().toString();
    const updatedHash = "1".repeat(63);
    const updatedLink = "updatedLink";

    await contractAccount.functionCall({
      contractId,
      methodName: "patch_contract_source_metadata",
      args: {
        new_source_metadata: {
          version: updatedVersion,
          commit_hash: updatedHash,
          link: updatedLink,
        },
      },
      gas,
      attachedDeposit: parseNearAmount("0.1"),
    });

    const source_metadata_updated = await contractAccount.viewFunction(
      contractId,
      "contract_source_metadata"
    );

    assert.strictEqual(source_metadata_updated.version, updatedVersion);
    assert.strictEqual(source_metadata_updated.commit_hash, updatedHash);
    assert.strictEqual(source_metadata_updated.link, updatedLink);
  });

  it("should allow the owner to update a single field of a contract's source_metadata", async function () {
    const source_metadata_original = await contractAccount.viewFunction(
      contractId,
      "contract_source_metadata"
    );

    const updatedVersion = Date.now().toString();

    await contractAccount.functionCall({
      contractId,
      methodName: "patch_contract_source_metadata",
      args: {
        new_source_metadata: {
          version: updatedVersion,
        },
      },
      gas,
      attachedDeposit: parseNearAmount("0.1"),
    });

    const source_metadata_updated = await contractAccount.viewFunction(
      contractId,
      "contract_source_metadata"
    );

    assert.strictEqual(source_metadata_updated.version, updatedVersion);
    assert.strictEqual(
      source_metadata_updated.commit_hash,
      source_metadata_original.commit_hash
    );
    assert.strictEqual(
      source_metadata_updated.link,
      source_metadata_original.link
    );
  });

  it("should allow the owner to mint an NFT", async function () {
    await owner.functionCall({
      contractId,
      methodName: "nft_mint",
      args: {
        token_id,
        metadata,
        token_type: tokenType,
        perpetual_royalties: {
          [bobId]: BOB_ROYALTY,
        },
      },
      gas,
      attachedDeposit: parseNearAmount("1"),
    });

    const tokens = await contractAccount.viewFunction(
      contractId,
      "nft_tokens",
      {
        from_index: "0",
        limit: 100,
      }
    );

    assert(tokens[tokens.length - 1].token_id == token_id);
  });

  it("should allow the owner to transfer the nft", async function () {
    await owner.functionCall({
      contractId,
      methodName: "nft_transfer",
      args: {
        receiver_id: aliceId,
        token_id,
      },
      gas,
      attachedDeposit: "1",
    });

    const { owner_id } = await contractAccount.viewFunction(
      contractId,
      "nft_token",
      { token_id }
    );
    assert.strictEqual(owner_id, aliceId);
  });

  it("should allow alice to list the token for sale", async function () {
    let sale_args = {
      sale_conditions: {
        near: parseNearAmount("1"),
      },
      token_type: tokenType,
      is_auction: false,
    };

    for (let i = 0; i < APPROVALS_TO_ATTEMPT; i++) {
      try {
        const nftApproveRes = await alice.functionCall({
          contractId: contractId,
          methodName: "nft_approve",
          args: {
            token_id,
            account_id: marketId,
            msg: JSON.stringify(sale_args),
          },
          gas,
          attachedDeposit: parseNearAmount("0.01"),
        });
      } catch (e) {
        // swallow and keep iterating
        console.warn(e);
      }
    }
  });

  it("should allow someone to buy the token and should have paid bob a royalty", async function () {
    const bobBalanceBefore = (await getAccountBalance(bobId)).total;

    const res = await contractAccount.functionCall({
      contractId: marketId,
      methodName: "offer",
      args: {
        nft_contract_id: contractId,
        token_id,
      },
      gas,
      attachedDeposit: parseNearAmount("1"),
    });

    const bobBalanceAfter = (await getAccountBalance(bobId)).total;

    assert.strictEqual(
      new BN(bobBalanceAfter).sub(new BN(bobBalanceBefore)).toString(),
      parseNearAmount("0.1")
    );
    const { owner_id } = await contractAccount.viewFunction(
      contractId,
      "nft_token",
      { token_id }
    );
    assert.strictEqual(owner_id, contractId);
  });

  it("should return payout object on call of nft_payout", async function () {
    const balanceInt = 1;
    const balance = parseNearAmount(balanceInt.toString());

    const res = await contractAccount.viewFunction(contractId, "nft_payout", {
      token_id,
      balance,
      max_len_payout: 9,
    });
    const bobExpected = (BOB_ROYALTY * balanceInt) / 10000;
    const contractAcctExpected = balanceInt - bobExpected;
    const expected = {
      [bobId]: bobExpected.toString(),
      [contractId]: contractAcctExpected.toString(),
    };
    for (let key in res.payout) {
      res.payout[key] = formatNearAmount(res.payout[key]);
    }
    assert.deepEqual(res.payout, expected);
  });
});
