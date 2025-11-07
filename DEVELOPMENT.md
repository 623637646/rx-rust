
# TODO

* Features  
  * 可以用过程宏实现：impl Disposable for Shared\<Mutable\<Context\>\>  
  * Async 转换 + Single 抽象（[最后验证操作符是不是全](https://reactivex.io/documentation/single.html)）  
    * Future 转 Single，可以删掉 Future 转 Observable  
    * Single 转 Future，可以删掉 Observable 转 Future（应该没有这个）  
* NotBigDeal  
  * 用 test_channel 代替 test 里的PublishSubject和create，just  
  * Infallible 替换为Never  
  * 去掉test_without_convenient_api

# Check List

1. 注释  
2. 文档 Example  
3. 合理使用（减少不必要）  
   1. pub，move  
   2. Clone，Send, Sync, 'static  
   3. lock/read 等锁。 检查是否需要用Context来用一个锁？不然可能有并发问题  
   4. 锁的使用  
      1. 尽量使用safe_lock宏  
      2. 检查锁是否及时释放  
         1. 在 on_next 时释放context锁  
         2. 在 on_termination 时，释放context锁和observer锁  
      3. 检查锁是否应该更大范围，即一个“事务”内，只锁一次。  
4. 单测   
   1. Cases:  
      1. test_completed  
      2. test_error  
      3. test_unsubscribe  
      4. test_ref  
      5. test_mut_ref  
      6. test_async  
      7. test_subscribe_by_different_observer  
      8. test_unsub_on_next_by_take  
      9. Non-creating observable  
         1. test_multiple_operation  
         2. test_without_convenient_api  
      10. Revertible observable  
          1. test_revert_completed  
          2. test_revert_error  
      11. Hot Observable (Subjects, RefCount)  
          1. test_complete_on_next  
          2. test_error_on_next  
          3. test_unsub_on_next  
          4. test_sub_on_next  
          5. test_next_on_next  
          6. test_unsub_on_completed  
          7. test_sub_on_completed  
          8. test_unsub_on_error  
          9. test_sub_on_error  
      12. Using lock(lock_mut|safe_lock.*!) 有的lock没 test_complete_after_next  
          1. test_immediate_next  
          2. test_immediate_completed  
          3. test_immediate_error  
      13. Using schedule(: Scheduler|::from_stream\\() 有的scheduler没有test_immediate_next （如 from future）  
          1. TESTS IN USING LOCK  
          2. test_complete_after_next  
          3. test_error_after_next  
          4. test_unsub_after_next  
          5. test_unsub_after_completed  
          6. test_unsub_after_error  
          7. test_order_with_continuous_next  
          8. test_no_delay (if appliable)  
          9. test_undisposed_scheduler (search this to find all module using scheduler)  
          10. test_scheduler_should_be_disposed_after_completed  
          11. test_scheduler_should_be_disposed_after_error  
          12. test_scheduler_should_be_disposed_after_unsub  
      14. Compiling checking  
          1. test_lifetime  
          2. test_fn  
          3. test_clone  
          4. test_type_inference_with_subscribe  
          5. test_type_inference_without_subscribe  
   2. 覆盖率

# Commend

* cargo fmt  
* cargo clippy --fix  
* cargo hack check --each-feature --no-dev-deps --exclude-all-features  
* reset; time RUST_BACKTRACE=1 cargo nextest run --no-fail-fast --failure-output final --status-level=fail --no-default-features --features tokio-scheduler  
  * fd7a1e528b7c241bcc21617c8c243d421fc6252f  
    * RUST_BACKTRACE=1 cargo nextest run --no-fail-fast --failure-output final      119.88s user 20.74s system 289% cpu 48.605 total  
  * 5b827281fe3626ce45fd0771e2c1ebadf05f2166  
    * RUST_BACKTRACE=1 cargo nextest run --no-fail-fast --failure-output final      183.86s user 28.95s system 372% cpu 57.055 total  
* reset; time RUST_BACKTRACE=1 cargo hack --each-feature --exclude-all-features --exclude-no-default-features --exclude-features default,multi-threaded,single-threaded nextest run --no-fail-fast --failure-output final --status-level=fail
* cargo test --doc --features tokio-scheduler
* cargo tarpaulin --out Html  
* cargo tarpaulin --out Html -- --test utils::unique_key_store  
* cargo doc --open  
* cargo expand