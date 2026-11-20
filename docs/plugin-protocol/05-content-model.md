# 05 · 内容模型

[← 总览](README.md)

本章定义插件与宿主之间传递的数据形状。字段的详细类型以后续产出的类型定义文件为准，
本章说明语义、可选性与长度上限。

## 1. 通用规则

| 规则 | 说明 |
| --- | --- |
| 标识 | 所有 `id` 都是插件作用域内的**不透明字符串**（1–256 字节）。宿主不解析其内容，只用于回传与本地记录 |
| 标识的稳定性 | `id` **必须**长期稳定：宿主用它做片库、历史与进度的键。不要把会过期的签名、令牌或时间戳放进 `id` |
| 时间 | 一律 Unix 毫秒 |
| 时长 | 一律秒（字段名以 `Sec` 结尾） |
| 文本 | 一律按纯文本渲染，唯一例外是标注为富文本的字段（§9） |
| 地址 | 宿主请求的资源为绝对 `https:` 地址，声明 `allowInsecureHttp` 时可用 `http:`；external 流的例外见 §7.3 |
| 可选项 | 除明确标注"必需"外都可省略。宿主按字段是否存在决定界面元素的显隐 |
| 长度上限 | 见 §8；展示文本截断，机器处理字段校验失败，超量列表条目丢弃 |

可选字段用省略表示“没有”；只有类型显式允许时才可返回 `null`，如分页 `next`。
数值字段传 `0` 表示“确实是 0”。标识不得截断；超长标识使所属条目无效，避免不同 ID 截成相同值。

## 2. 媒体类型

```ts
type MediaType =
  | "movie"        // 电影
  | "series"       // 剧集
  | "anime"        // 番剧
  | "variety"      // 综艺
  | "documentary"  // 纪录片
  | "short"        // 短剧、短视频
  | "live"         // 直播
  | "other";
```

类型影响宿主界面的默认形态，但分集选择器最终按实际分集与播放列表数量显示，不能仅因 movie 隐藏多分集。
插件**可以**在条目上省略 `type`，
宿主按 `other` 处理；未知媒体类型也降级为 `other`。一个插件可以同时提供多种类型。

## 3. 媒体条目（`MediaItem`）

列表卡片、搜索结果、片库条目都用它。

| 字段 | 必需 | 类型 | 说明 |
| --- | --- | --- | --- |
| `id` | 是 | string | 插件内的唯一标识 |
| `title` | 是 | string | 标题 |
| `type` | 否 | MediaType | 媒体类型 |
| `cover` | 否 | ImageRef | 竖版封面 |
| `backdrop` | 否 | ImageRef | 横版图，用于详情页头部 |
| `description` | 否 | string | 简介 |
| `tags` | 否 | string[] | 标签 |
| `score` | 否 | number | 评分，0–10 |
| `year` | 否 | number | 年份 |
| `status` | 否 | `"ongoing" \| "finished" \| "upcoming"` | 更新状态 |
| `episodeCount` | 否 | number | 已更新集数 |
| `totalEpisodes` | 否 | number | 计划总集数 |
| `durationSec` | 否 | number | 单集时长 |
| `stats` | 否 | `{ plays?, hot?, likes?, favorites?, comments? }` | 展示用计数 |
| `author` | 否 | Author | 作者、UP 主、制作方 |
| `publishedAt` | 否 | number | 发布时间 |
| `updatedAt` | 否 | number | 最近更新时间 |
| `badge` | 否 | string | 角标短文本，≤ 8 字符（如"原创"） |
| `entryEpisodeId` | 否 | string | 沉浸式信息流的直接播放入口：给了它，宿主无需先取详情即可解析播放 |

```ts
interface Author {
  id: string;
  name: string;
  avatar?: ImageRef;
  description?: string;
  followers?: number;
}
```

## 4. 详情（`MediaDetail`）

在 `MediaItem` 的基础上增加：

| 字段 | 必需 | 说明 |
| --- | --- | --- |
| `playlists` | 是 | 分集列表，至少一项（§5） |
| `seasons` | 否 | 季的展示名：`{ number, label }[]`。分集带 `season` 时，宿主按季分组并用这里的名称 |
| `people` | 否 | 演职员：`{ name, role?, avatar? }[]` |
| `trailers` | 否 | 预告片：`Stream[]`（§7.3） |
| `interaction` | 否 | 当前用户的状态：`{ liked?, favorited?, likedEpisodeIds? }` |
| `related` | 否 | 相关推荐 |

电影、单集内容也返回 `playlists`，只是里面只有一集。

## 5. 分集列表（`Playlist`）与分集（`Episode`）

同一个作品的多个片源、多个配音、正片与花絮，各是一个 `Playlist`。

```ts
interface Playlist {
  id: string;
  label?: string;                    // 如"线路 1""国语""花絮"
  kind?: "main" | "dub" | "extra" | "other";   // 默认 main
  episodes: Episode[];
  next?: string | null;              // 分集未一次给完时的游标（配合 listEpisodes）
}

interface Episode {
  id: string;
  index: number;                     // 从 1 开始的正整数；同一 Playlist 内唯一的列表排序序号
  season?: number;                   // 季号，非负整数，0 可用于特别篇；给出后宿主按季分组展示
  episodeNumber?: number;            // 真实集号，非负有限数字，可为 12.5；用于匹配，不等于排序 index
  label?: string;                    // 展示用编号文本，如 "12"、"SP1"、"12.5"；省略时用 index
  title?: string;
  durationSec?: number;
  airDate?: number;                  // 播出时间
  thumbnail?: ImageRef;
  locked?: boolean;                  // 当前不可播放
  lockReason?: string;               // 置灰时展示的原因
}
```

- `season`、`episodeNumber` 表达真实季集编号，`index` 只表达当前播放列表的顺序，不负责跨来源身份匹配。
- 切换播放列表时优先按明确的季号、集号匹配；无明确编号时可以用 `index` 提议候选，
  但不得把不同剪辑、正片与花絮、拆集版视为确定匹配。存在歧义时让用户选集。
- 分集数量很大（如数千集的直播回看）时，插件可以在 `playlists[].episodes` 里只给第一页，
  并用 `next` 配合 `media.listEpisodes` 能力分页（见 [06](06-capabilities.md)）。

## 6. 分页

```ts
interface PageParams { cursor?: string; limit?: number }   // limit 是宿主的期望条数提示（1–100）

interface Page<T> {
  items: T[];
  next?: string | null;     // 下一页游标；null 或省略表示没有更多
  total?: number;           // 已知总数（可选，仅用于展示）
}
```

游标对宿主不透明，插件可以随意编码（页码型上游把页码写进游标即可）。
游标**不得**依赖插件的内存状态（见 [02 §5.4](02-runtime.md)）。

## 7. 资源引用与播放

### 7.1 图片与其它资源

```ts
type ImageRef = string | ResourceRef;

interface ResourceRef {
  url: string;
  headers?: Record<string, string>;      // 取回时附带的请求头
  transform?: ResourceTransform;         // 取回后的字节变换
}
```

直接给字符串地址是最常见的形式；需要自定义请求头（防盗链）或解密时用对象形式。

### 7.2 字节变换

部分站点的封面、分片不是标准图片或视频，需要解密。

```ts
type ResourceTransform =
  | { type: "aes-cbc"; key: string; iv: string; padding?: "pkcs7" | "none" }  // key/iv 为 Base64
  | { type: "plugin"; name: string; state?: Json };
```

- `aes-cbc` 由宿主原生执行，不占用插件运行时，适合封面这类高频小资源。
- `plugin` 由宿主调用插件的 `transformResource`（见 [06](06-capabilities.md)），
  `name` 与 `state` 原样回传。适合非标准算法。
- 分片变换需要宿主支持特性 `transform.segment`。
- 变换对象以完整资源为单位，不是流式回调。输入与输出必须分别满足 8 MiB 的整个调用大小上限；
  宿主在完整读入前设置资源大小预算，并为 URL、state 等其他字段预留空间。
  1.0 不支持对整个 progressive 视频执行插件变换，也不承诺任意大小的分片都可变换。
  媒体范围请求与变换顺序见 [09 §4](09-media-transport.md)。

### 7.3 播放信息

```ts
interface PlaybackInfo {
  streams: Stream[];              // 至少一条；宿主结合设备能力与用户设置挑选
  servers?: PlaybackServer[];     // 可切换的播放线路
  subtitles?: SubtitleTrack[];
  expiresAt?: number;             // 本次播放信息失效时间；刷新沿宿主记录的解析链进行（09 §5）
  durationSec?: number;
  resumeSec?: number;             // 上游记录的续播位置（宿主本地记录优先）
  skip?: { introEndSec?: number; outroStartSec?: number };
}

interface PlaybackServer {
  id: string;
  label?: string;
}

interface Stream {
  url: string;
  kind?: "media" | "page" | "external";    // 默认 media
  format?: "hls" | "dash" | "progressive"; // kind 为 media 时必需
  codec?: "h264" | "hevc" | "av1";
  label?: string;                          // 如 "1080P"
  height?: number;
  bitrate?: number;                        // bit/s
  serverId?: string;                       // 属于哪条播放线路
  live?: boolean;
  headers?: Record<string, string>;        // 取清单、密钥、分片时附带的请求头
  resolverId?: string;                     // kind 为 page 时，指定用哪个解析器插件
  transforms?: {
    playlist?: ResourceTransform;
    segment?: ResourceTransform;
    key?: ResourceTransform;
  };
  external?: { app?: string; mime?: string };   // kind 为 external 时的提示信息
}
```

三种 `kind` 的含义：

| `kind` | 含义 | 宿主行为 |
| --- | --- | --- |
| `media` | 可直接播放的媒体地址 | 交给播放器 |
| `page` | 只有播放页，需要解析 | 交给已安装的解析器插件（[06 §3](06-capabilities.md)）；没有可用解析器时提示用户 |
| `external` | 无法在应用内播放（磁力链、外部站点） | 询问用户后交给系统或其他应用打开 |

`external.url` 是外部打开地址，允许 HTTP(S)、`magnet:`、`ed2k:`、`rtsp:`、`rtmp:`；
HTTP 在此仅交给外部应用，不因 `allowInsecureHttp` 关闭而被当作宿主网络请求拒绝。
宿主可以因平台没有处理程序而提示不支持；不得把这些地址交给媒体代理请求，也不接受 `javascript:`、`data:`、`file:`。
其他业务资源仍遵循 HTTP(S) 规则。外部打开不传递宿主保存的 Cookie、请求头或变换。

**播放线路**：一集有多个取流服务器时，插件在 `servers` 里列出它们，并在 `streams` 里给出
默认（或被请求的）那一条线路的流。用户切换线路时，宿主带上 `serverId` 重新调用 `resolvePlayback`。
这样展示线路列表不需要把每条线路都请求一遍。

### 7.4 请求头

媒体请求（图片、播放列表、密钥、分片、字幕）的请求头按以下优先级合并，高优先级覆盖低优先级：

| 优先级 | 来源 |
| --- | --- |
| 1 | 条目或流自身的 `headers`（`ResourceRef.headers`、`Stream.headers`、`SubtitleTrack.headers`） |
| 2 | 插件的 `meta.defaultHeaders`（见 [01 §2.1](01-plugin-file.md)） |
| 3 | 宿主的默认值：`User-Agent` 为 `HGhub/<版本>`，其余不设 |

- 影视源里同一站点的图片与流通常需要同一个 `Referer`，用默认值声明即可，不必逐条重复。
- 这三层都**不得**覆盖由传输层决定的请求头（[03 §2](03-host-rules.md)）。
- **同源规则对每一层都生效**：`Cookie` 与 `Authorization` 无论来自哪一层，都只发给与资源地址
  （`Stream.url` 或 `ResourceRef.url`）**同源**的地址，不随清单转发到第三方 CDN。
  默认头不能绕过这条规则——否则一个默认的 `Referer` 之类的头就可能把凭据带到 CDN 上。
- 媒体请求的 `Range` 由播放器与宿主控制，插件与默认头都不得指定。
- 请求头名称大小写不敏感。媒体请求按资源提供者的 Cookie 罐正常匹配目标 URL；显式 Cookie 头覆盖自动 Cookie。
  自动 Cookie 不跨插件借用，跨域资源只携带其自身匹配到的 Cookie。资源提供者及解析器合并规则见 [09](09-media-transport.md)。

### 7.5 字幕轨

```ts
interface SubtitleTrack {
  url: string;
  format: "vtt" | "srt" | "ass";
  language?: string;      // BCP 47
  label?: string;
  default?: boolean;
  headers?: Record<string, string>;
}
```

宿主负责 SRT → VTT 转换与编码识别；`ass` 轨是否渲染由宿主决定，不支持时忽略该轨。

## 8. 长度上限

宿主**必须**在超限时截断展示文本或丢弃超量列表条目，并记录一条警告；标识、游标、地址、密钥等
机器处理字段不得截断，非法时使所属条目或调用失败。列表截断不能伪造 next 游标；分页完整性由插件保证。

| 字段 | 上限 |
| --- | --- |
| `title`、`name`、`label` | 200 字符 |
| `description`、`summary` | 5000 字符 |
| `badge` | 8 字符 |
| 单个 `tag` | 30 字符 |
| `tags` | 20 项 |
| 地址 | 8 KiB |
| 富文本正文 `contentHtml` | 512 KiB |
| 单页条目 | 100 |
| `playlists` | 20 |
| 单个 `playlists[].episodes` | 5000 |
| `streams` | 16 |
| `servers` | 20 |
| `subtitles` | 32 |

## 9. 富文本

图文类内容可以返回 HTML：

- 宿主**必须**先清洗再渲染：只保留基本排版、链接与图片，移除脚本、样式、事件属性、
  内联样式、`iframe` 与表单；链接一律交由外部浏览器打开。
- 正文中的图片地址由宿主改写为本地媒体协议，同样受网络规则约束。
- 插件**应当**只返回正文片段（如 `<p>`、`<img>`），不要返回整页 HTML。
