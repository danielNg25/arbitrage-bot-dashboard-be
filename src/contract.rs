use alloy::sol;

sol! {
    #[sol(rpc)]
    IUniversalRouter,
    "abis/IUniversalRouter.json"
}

sol! {
    #[sol(rpc)]
    IUniswapV3Factory,
    "abis/IUniswapV3Factory.json"
}

sol! {
    #[sol(rpc)]
    IUniswapV2Factory,
    "abis/IUniswapV2Factory.json"
}

sol! {
    #[sol(rpc)]
    IAlgebraFactory,
    "abis/IAlgebraFactory.json"
}
