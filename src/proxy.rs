pub async fn serve(state: crate::StateRef) -> Result<(), crate::rgpt::Error> {
    let session = crate::rgpt::alloc_session(state).await?;
    println!("Session: {:?}", session);
    Ok(())
}
