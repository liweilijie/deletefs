# detetefs
利用一个**多线程**来实现的，因为 **rename** 本来也不耗时，利用`5T` 的数据测试下来性能足够了。

- 删除多余的重复的文件。( 删除实际上是 `remove` 到 **trash** 目录里面 )
  > deletefs -p "directory" del
- rename 文件名。
  > deletefs -p "directory" trim
  >
  > deletefs -p "directory" trim "你想要删除的名称内容"
