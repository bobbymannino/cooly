use cooly_api::ApiError;
use gpui_kit::{Context, Task};

/// Runs a blocking API call on a background thread, then hands the result to the view on the UI thread.
///
/// Dropping the returned task cancels the hand-off, so views keep it in a field while a request is in flight.
pub fn spawn_api_call<V: 'static, T: Send + 'static>(
    cx: &Context<V>,
    call: impl FnOnce() -> Result<T, ApiError> + Send + 'static,
    done: impl FnOnce(&mut V, Result<T, ApiError>, &mut Context<V>) + 'static,
) -> Task<()> {
    let call = cx.background_executor().spawn(async move { call() });
    cx.spawn(async move |this, cx| {
        let result = call.await;
        // Fails only if the view was closed meanwhile, in which case there's nothing left to update.
        this.update(cx, |view, cx| done(view, result, cx)).ok();
    })
}
