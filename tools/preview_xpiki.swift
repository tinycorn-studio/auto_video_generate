import AppKit

// Static design preview only. No application logic or API requests.
let outDir = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "previews"
try FileManager.default.createDirectory(atPath: outDir, withIntermediateDirectories: true)
let canvasWidth: CGFloat = 1240
let canvasHeight: CGFloat = 1200
func col(_ hex: String) -> NSColor {
    let v = Int(hex, radix: 16)!
    return NSColor(srgbRed: CGFloat((v >> 16) & 255)/255, green: CGFloat((v >> 8) & 255)/255, blue: CGFloat(v & 255)/255, alpha: 1)
}
let page = col("F3F4F7"), white = NSColor.white, ink = col("1B1E27"), muted = col("747B8B")
let line = col("E5E7ED"), purple = col("5358D8"), lavender = col("EDEDFF"), green = col("358B71")
func box(_ x: CGFloat, _ y: CGFloat, _ w: CGFloat, _ h: CGFloat, _ fill: NSColor, _ radius: CGFloat = 12, stroke: NSColor? = nil) {
    let p = NSBezierPath(roundedRect: NSRect(x:x,y:y,width:w,height:h), xRadius:radius,yRadius:radius)
    fill.setFill(); p.fill()
    if let stroke = stroke { stroke.setStroke(); p.lineWidth = 1; p.stroke() }
}
func txt(_ s: String, _ x: CGFloat, _ y: CGFloat, _ size: CGFloat = 13, _ color: NSColor = ink, _ weight: NSFont.Weight = .regular, _ width: CGFloat = 1000, _ height: CGFloat = 100) {
    let style = NSMutableParagraphStyle(); style.lineSpacing = 4
    (s as NSString).draw(in: NSRect(x:x,y:y,width:width,height:height), withAttributes:[.font:NSFont.systemFont(ofSize:size,weight:weight),.foregroundColor:color,.paragraphStyle:style])
}
func button(_ s: String, _ x: CGFloat, _ y: CGFloat, _ w: CGFloat, filled: Bool = false) {
    box(x,y,w,35,filled ? purple : white,8,stroke:filled ? nil : line)
    txt(s,x+12,y+9,12,filled ? white : purple,.semibold,w-20,22)
}
func pill(_ s: String, _ x: CGFloat, _ y: CGFloat, _ w: CGFloat, selected: Bool = true) {
    box(x,y,w,29,selected ? lavender : white,7,stroke:selected ? col("C8C9F5") : line)
    txt(s,x+10,y+7,11,selected ? purple : muted,.medium,w-16,20)
}
func wave(_ x: CGFloat, _ y: CGFloat, _ width: CGFloat, _ height: CGFloat, _ color: NSColor) {
    let p = NSBezierPath(); p.lineWidth = 2; p.lineCapStyle = .round
    for i in 0..<51 {
        let amplitude = CGFloat(4 + ((i * 17 + i * i) % 31)) / 35 * height
        let px = x + CGFloat(i) * width / 50
        p.move(to:NSPoint(x:px,y:y-amplitude/2)); p.line(to:NSPoint(x:px,y:y+amplitude/2))
    }
    color.setStroke(); p.stroke()
}
func fileCard(_ title: String, _ detail: String, _ icon: String, _ y: CGFloat, _ tint: NSColor) {
    box(54,y,514,61,white,11,stroke:line)
    box(66,y+11,38,38,tint,10)
    txt(icon,77,y+18,20,purple,.medium,30,30)
    txt(title,116,y+13,13,ink,.semibold,340,24)
    txt(detail,116,y+35,11,muted,.regular,340,20)
    txt("Đổi file",507,y+23,11,purple,.semibold,55,20)
}
func video() {
    let x: CGFloat = 772, y: CGFloat = 235, w: CGFloat = 258, h: CGFloat = 459
    let clip = NSBezierPath(roundedRect:NSRect(x:x,y:y,width:w,height:h),xRadius:14,yRadius:14)
    NSGraphicsContext.saveGraphicsState(); clip.addClip()
    let grad = NSGradient(colors:[col("253A58"),col("8F8793"),col("E8BB95"),col("434B67")])!
    grad.draw(in:NSRect(x:x,y:y,width:w,height:h),angle:90)
    box(x+155,y+75,49,49,col("F8D4A2"),25)
    let hills = NSBezierPath()
    hills.move(to:NSPoint(x:x-10,y:y+h)); hills.line(to:NSPoint(x:x-10,y:y+250))
    hills.curve(to:NSPoint(x:x+w+10,y:y+320),controlPoint1:NSPoint(x:x+85,y:y+180),controlPoint2:NSPoint(x:x+140,y:y+340))
    hills.line(to:NSPoint(x:x+w+10,y:y+h)); hills.close(); col("37465E").setFill(); hills.fill()
    let hills2 = NSBezierPath()
    hills2.move(to:NSPoint(x:x-10,y:y+h)); hills2.line(to:NSPoint(x:x-10,y:y+335))
    hills2.curve(to:NSPoint(x:x+w+10,y:y+280),controlPoint1:NSPoint(x:x+90,y:y+390),controlPoint2:NSPoint(x:x+150,y:y+230))
    hills2.line(to:NSPoint(x:x+w+10,y:y+h)); hills2.close(); col("253448").setFill(); hills2.fill()
    txt("MỘT CHÚT BÌNH YÊN",x+32,y+302,14,col("FFFFFF"),.semibold,220,30)
    txt("Giữa những ngày vội vã",x+39,y+330,11,col("C8CFDB"),.regular,220,25)
    wave(x+20,y+399,w-40,31,col("75E8E1"))
    NSGraphicsContext.restoreGraphicsState()
    txt("▶",x+8,y+h+16,13,purple,.medium,20,22)
    txt("00:00 / 08:24",x+35,y+h+17,11,muted,.regular,150,22)
    txt("9:16 · Full HD",x+174,y+h+17,10,muted,.regular,120,22)
}
func screen(_ result: Bool) {
    box(0,0,canvasWidth,canvasHeight,page,0)
    box(0,0,canvasWidth,68,white,0)
    box(44,18,32,32,lavender,10); wave(51,34,18,15,purple)
    txt("Wavely",86,23,20,ink,.bold)
    txt("●",911,27,10,green); txt("Engine sẵn sàng",928,25,12,muted)
    pill("✦  Gemini · Xpiki",1053,20,145)
    txt("TẠO VIDEO · 01",54,95,11,purple,.bold)
    txt("Một ảnh, một giai điệu",54,119,29,ink,.bold)
    txt("Ghép bản thu thành video, chuẩn bị nội dung để chia sẻ.",54,161,13,muted)
    txt("Ảnh nền",54,206,13,ink,.semibold)
    txt("Ảnh sẽ cắt theo 9:16",435,208,11,muted)
    fileCard("binh-yen-hoang-hon.jpg","2400 × 3600 · JPG", "▧", 231, lavender)
    txt("Âm thanh",54,310,13,ink,.semibold)
    txt("Video theo độ dài bản thu",411,312,11,muted)
    fileCard("mot-chut-binh-yen.mp3","08 phút 24 giây · MP3", "♪", 335, col("FFF3DF"))
    txt("Khổ video",54,418,13,ink,.semibold)
    pill("▯  Dọc · 9:16",54,445,150)
    pill("▭  Ngang · 16:9",213,445,160,selected:false)
    txt("Kiểu sóng âm",54,496,13,ink,.semibold)
    pill("⌁  Tối giản",54,523,150)
    pill("╫  Cân xứng",213,523,160,selected:false)
    pill("▥  Spectrum",382,523,186,selected:false)
    box(54,568,514,43,white,9,stroke:line)
    txt("Vị trí: Dưới giữa",67,582,11,muted,.medium)
    txt("Kích thước: 80%",308,582,11,muted,.medium)
    txt("Chỉnh  ›",507,581,11,purple,.semibold)

    box(54,635,514,390,white,14,stroke:col("D7D8EE"))
    box(70,652,31,31,lavender,9); txt("✦",78,657,20,purple,.medium,30,30)
    txt("Tạo nội dung đăng bài",112,652,15,ink,.semibold)
    txt("Kèm tiêu đề, mô tả và hashtag",112,677,11,muted)
    box(505,656,45,25,purple,13); box(528,659,19,19,white,10)
    txt("Xpiki API key",72,710,11,ink,.semibold)
    txt("Gemini 3.8 Flash · Xpiki",390,710,10,muted)
    box(72,733,392,35,col("F8F9FB"),8,stroke:line)
    txt("••••••••••••••••••••••••",84,744,12,muted,.regular,290,20)
    txt("◉",435,741,13,muted,.regular,20,22)
    button("Lưu",474,733,76,filled:true)
    txt("✓ Đã lưu trên máy",72,776,10,green,.medium)
    txt("Ngôn ngữ",72,803,11,ink,.semibold)
    txt("Nền tảng",267,803,11,ink,.semibold)
    pill("English     ⌄",72,826,177,selected:false)
    pill("✓ YouTube",267,826,105)
    pill("✓ TikTok",380,826,85)
    pill("+ Khác",473,826,77,selected:false)
    txt("Bản chép lời / tóm tắt nội dung",72,874,11,ink,.semibold)
    txt("(bắt buộc)",382,874,10,muted)
    box(72,896,478,52,col("F8F9FB"),7,stroke:line)
    txt("Dán transcript hoặc tóm tắt chính xác nội dung âm thanh…",83,907,11,muted,.regular,450,36)
    txt("Xpiki Gemini chỉ nhận văn bản. Wavely không gửi âm thanh hoặc video.",72,962,9,muted,.regular,480,36)

    box(54,1038,514,46,purple,10)
    txt(result ? "↻   Tạo video mới" : "▶   Tạo video & nội dung đăng bài",result ? 245 : 196,1052,13,white,.semibold,400,25)
    txt("Lưu cùng thư mục: video.mp4 + video-noi-dung.txt",121,1095,10,muted)

    txt("XEM TRƯỚC",640,105,11,muted,.bold)
    txt("BẢN DEMO GIAO DIỆN",1040,105,9,muted,.medium)
    box(640,140,546,42,col("E7E9EF"),10)
    box(result ? 914 : 644,144,268,34,white,8)
    txt("Video",755,153,12,result ? muted : purple,.semibold,100,24)
    txt("Nội dung đăng bài",989,153,12,result ? purple : muted,.semibold,170,24)

    if !result {
        video()
        txt("Bố cục xem trước · Sóng sẽ bám theo âm thanh thực",726,758,10,muted,.regular,420,25)
        box(666,813,494,157,white,13,stroke:line)
        box(683,831,35,35,lavender,9); txt("≡",694,838,19,purple,.semibold,28,28)
        txt("Sẵn sàng cho lần đăng tiếp theo",731,833,14,ink,.semibold,410,28)
        txt("Sau khi xuất video, Wavely sẽ tạo thêm:",731,859,11,muted,.regular,410,25)
        txt("✓  Tiêu đề và mô tả riêng cho từng nền tảng\n✓  Hashtag và từ khóa liên quan\n✓  Một file .txt lưu cạnh video",731,887,12,ink,.regular,405,90)
        txt("Bạn có thể xem, sửa và sao chép trước khi đăng.",697,991,11,muted,.regular,470,40)
    } else {
        box(640,202,546,63,col("E7F4EE"),10)
        txt("✓",656,220,20,green,.semibold,30,30)
        txt("Video và nội dung đã sẵn sàng",688,215,14,green,.semibold,460,25)
        txt("mot-chut-binh-yen.mp4 · mot-chut-binh-yen-noi-dung.txt",688,240,10,green,.regular,475,25)
        pill("YouTube",640,286,98)
        pill("TikTok",748,286,86,selected:false)
        txt("NỘI DUNG DO AI GỢI Ý · CÓ THỂ CHỈNH SỬA",640,340,9,muted,.semibold)
        box(640,367,546,118,white,12,stroke:line)
        txt("TIÊU ĐỀ",657,384,10,muted,.semibold)
        txt("Sao chép",1101,384,10,purple,.semibold,80,20)
        txt("A quiet moment to unwind | 8 minutes\nto slow down and listen",657,413,18,ink,.semibold,505,70)
        box(640,501,546,262,white,12,stroke:line)
        txt("MÔ TẢ",657,519,10,muted,.semibold)
        txt("Sao chép",1101,519,10,purple,.semibold,80,20)
        txt("Some days, all we need is a little stillness. Take a\nfew minutes to relax, breathe, and return to your own\nrhythm.\n\nPut on your headphones, find a quiet corner, and listen.\n\nIf this recording brings you a moment of calm, save it\nfor later or share it with someone you love.",657,549,13,ink,.regular,505,205)
        box(640,779,546,87,white,12,stroke:line)
        txt("HASHTAG",657,795,10,muted,.semibold)
        txt("Sao chép",1101,795,10,purple,.semibold,80,20)
        txt("#QuietMoments  #Relax  #SlowLiving  #MindfulListening",657,827,13,purple,.medium,500,25)
        txt("Keywords: quiet instrumental, relaxation, calm moments, slow living",640,885,11,muted,.regular,545,30)
        button("Sao chép tất cả",640,930,164,filled:true)
        button("↗  Mở file .txt",814,930,157)
        button("Mở thư mục",981,930,154)
        txt("Based on your transcript/summary · Edit in the .txt file before posting.",640,986,11,muted,.regular,545,40)
    }
    box(0,1170,canvasWidth,30,white,0)
    txt("Video được xử lý trên máy · Xpiki chỉ nhận transcript/tóm tắt khi bật tạo nội dung",44,1179,9,muted)
    txt("Wavely · Thiết kế đề xuất",1061,1179,9,muted)
}
for (name, result) in [("Wavely-Xpiki-Setup-Preview.png",false),("Wavely-Xpiki-Content-Preview.png",true)] {
    let img = NSImage(size:NSSize(width:canvasWidth*2,height:canvasHeight*2))
    img.lockFocusFlipped(true)
    NSGraphicsContext.current!.cgContext.scaleBy(x:2,y:2)
    screen(result)
    img.unlockFocus()
    let rep = NSBitmapImageRep(data:img.tiffRepresentation!)!
    let url = URL(fileURLWithPath:outDir).appendingPathComponent(name)
    try rep.representation(using:.png,properties:[:])!.write(to:url)
    print(url.path)
}
