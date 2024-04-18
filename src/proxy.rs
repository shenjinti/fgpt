pub async fn serve(state: crate::StateRef) -> Result<(), crate::fgpt::Error> {
    let session = crate::fgpt::alloc_session(state).await?;
    println!("Session: {:?}", session);
    Ok(())
}
