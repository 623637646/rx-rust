
# TODO

* Features  
  * Async 转换 + Single 抽象（[最后验证操作符是不是全](https://reactivex.io/documentation/single.html)）  
    * Future 转 Single，可以删掉 Future 转 Observable  
    * Single 转 Future，可以删掉 Observable 转 Future（应该没有这个）  
* NotBigDeal  
  * 用 test_channel 代替 test 里的PublishSubject和create，just  
  * Infallible 替换为Never  

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
      11. Hot Observable (e.g. Subject, ConnectableObservable, RefCount. Only those that can borrow sender or own sender at the same time)  
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
          1. test_next_on_sub  
          2. test_complete_on_sub  
          3. test_error_on_sub  
          4. test_sub_on_sub (TODO)
          5. test_next_on_unsub (TODO)
          6. test_complete_on_unsub (TODO)
          7. test_error_on_unsub (TODO)
          8. test_sub_on_unsub (TODO)
          9. test_race_condition (WIP)
      13. Using schedule(: Scheduler|::from_stream\\() 有的scheduler没有test_next_on_sub （如 from future）  
          1. ALL TESTS IN "Using lock"  
          2. test_complete_after_next  
          3. test_error_after_next  
          4. test_unsub_after_next  
          5. test_unsub_after_completed  
          6. test_unsub_after_error  
          7. test_order_with_continuous_next  
          8. test_no_delay (if appliable)  
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
* cargo tarpaulin --out Html --features tokio-scheduler  
* cargo tarpaulin --out Html -- --test utils::unique_key_store  
* cargo doc --open  
* cargo expand

# Others

正则表达式：锁的使用
lock_mut|lock_ref|safe_lock\w*!|\Wread\(\)|\Wwrite\(|\Wchange_if_not_equal\(
194个结果
排除路径：src/utils/safe_lock.rs,src/utils/types.rs
