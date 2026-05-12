import { createUmi } from '@metaplex-foundation/umi-bundle-defaults';
import { generateSigner, keypairIdentity, some, none } from '@metaplex-foundation/umi';
import { Keypair as Web3Keypair } from '@solana/web3.js';
import { createTreeV2, mintV2, mplBubblegum, parseLeafFromMintV2Transaction } from '@metaplex-foundation/mpl-bubblegum';

const HELIUS = `https://devnet.helius-rpc.com/?api-key=${process.env.HELIUS_API_KEY}`;
const fs = require('fs');

async function main() {
  const umi = createUmi(HELIUS).use(mplBubblegum());
  const raw = JSON.parse(fs.readFileSync(`${process.env.HOME}/.config/solana/id.json`, 'utf8')) as number[];
  const w3 = Web3Keypair.fromSecretKey(Uint8Array.from(raw));
  const payerKp = umi.eddsa.createKeypairFromSecretKey(w3.secretKey);
  umi.use(keypairIdentity(payerKp));
  console.log('payer:', payerKp.publicKey);

  const tree = generateSigner(umi);
  console.log('creating V2 tree:', tree.publicKey);
  const t = await (await createTreeV2(umi, { merkleTree: tree, maxDepth: 5, maxBufferSize: 8, public: false })).sendAndConfirm(umi);
  console.log('  tree tx ok');

  // Wait, mintV2 NO coreCollection
  await new Promise(r => setTimeout(r, 5000));
  const leafOwner = generateSigner(umi);
  console.log('mintV2 WITHOUT coreCollection...');
  try {
    const m = await mintV2(umi, {
      leafOwner: leafOwner.publicKey,
      merkleTree: tree.publicKey,
      metadata: {
        name: 'NoCol',
        uri: 'https://example.invalid/x.json',
        sellerFeeBasisPoints: 0,
        collection: none(),
        creators: [],
      },
    }).sendAndConfirm(umi);
    console.log('  ✅ mintV2 no-collection SUCCESS');
    const leaf = await parseLeafFromMintV2Transaction(umi, m.signature);
    console.log('  leaf id:', leaf.id);
  } catch (e: any) {
    console.log('  ❌ mintV2 no-collection FAILED:', e.message?.split('\n')[0]);
    if (e.transactionLogs) console.log('  logs:', e.transactionLogs.filter((l: string)=>l.includes('log:')).slice(0,5));
  }
}
main().catch(e => { console.error(e); process.exit(1); });
