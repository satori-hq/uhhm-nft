# HipHopHeads contract

## Branch/Versioning overview

This project was originally deployed by Satori in June 2021 using contracts (NFT & auction-style market) written by Matt Lockyer and Vadim Ilin, with the marketplace UI hosted at [nft.hiphop](https://nft.hiphop).

No upgrades were made until June 28, 2022. At this time, an upgrade was made to support marketplace listings of HHH NFTs (primarily Few and Far marketplace)

Branches have been frozen at deployments and are outlined below:

**`deployed-version-6-22-21` - deployed 6/22/21**

- Matt/Vadim deployed

**`upgrade-#1-6-28-22` - deployed 6/28/22, calling `migrate` method on deploy**

- Upgrades Payout functionality to integrate with standard market contracts
- Tested against `/contracts/market-new`, which is same sample marketplace contract that we test `nft-series` NFT contract against
- **IMPORTANT:** Auction marketplace contract that was used to manage original HHH auction sale has been moved to `market-old`, and has NOT been upgraded as it is assumed this contract is no longer in use now that all NFTs have been minted and https://nft.hiphop is no longer functional.

**`upgrade-#2-6-28-22` - deployed 6/28/22, calling `migrate` method on deploy**

- Adds Satori royalty at contract level, plus a Satori royalty cap (set at 2.5%), Satori account ID (snft.near) and a setter method
- Initiates contract with 2.5% Satori royalty
