# item dao

A dao for managing 'items'.


## Addresses

### token program:
Initial mint of 100,000 tokens. Unmodified cw20-base contract.

- address: `juno1slz6c85kxp4ek5ufmcakfhnscv9r2snlemxgwz6cjhklgh7v2hmsnwzy2k`
- code_id: `23`

### dao program v0.1
Does nothing. Just tests that I understand how passing arguments into
queries and executions work.

- code_id: `28`
- address: `juno14q5elxj4ghktt7d7d0uw0cs0gqyeay25h5fkree897gjm38gevxqexk8fn`

### dao program v0.2
Allows creation of proposals and voting on those proposals. All coins
sent to contract are burned. Features a bug where enums were not
converted to snake case during serialization.

- code_id: `59`
- address: `juno1cxnmukp9tjxksduey6wkgqmwzuvhvqqcwqq54pfh45xt7ze9kmjsjhng0u`

## dao program v0.3
Same as v0.2 but with the snake case bug fixes.

- code_id: `60`
- address: `juno10wmp4czgnww9tvwg7xu239k88vx4ksyqwah245nsaj7lz6r9v6uqdepwc2`


## Misc reference

Example execute message to create a new proposal:

```
'{"propose":{"title":"all twitter profile photos should be 🦄s!","body":"by making all of our twitter profile photos 🦄 emojis it will be clear that we are part of a top secret club.","action":{"add_item":{"name":"🦄 twitter profile photos required","contents":"all twitter profiles shall be 🦄s"}}}}'
```

Example query to view a proposal

```
junod query wasm contract-state smart juno10wmp4czgnww9tvwg7xu239k88vx4ksyqwah245nsaj7lz6r9v6uqdepwc2 '{"get_proposal":{"proposal_id": 0}}'
```
