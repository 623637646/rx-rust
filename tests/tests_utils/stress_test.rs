use crate::tests_utils::test_runtime::{TestRuntime, block_on};
use rx_rust::utils::types::NecessarySend;

pub(crate) fn stress_test<FU>(
    test: impl FnOnce(TestRuntime, usize) -> FU + Clone + NecessarySend + 'static,
) where
    FU: Future<Output = ()> + NecessarySend + 'static,
{
    block_on(|runtime| async move {
        const COUNT: usize = 100000;
        let mut handles = Vec::with_capacity(COUNT);
        for i in 0..COUNT {
            let test = test.clone();
            let runtime_cloned = runtime.clone();
            let handle = runtime.spawn(async move {
                test(runtime_cloned, i).await;
            });
            handles.push(handle);
        }
        for handle in handles {
            handle.await.unwrap();
        }
    });
}
