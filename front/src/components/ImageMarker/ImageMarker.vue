<template>
  <div
    class="image_marker_container"
    @ratechange="handleRateChange"

  >
    <img
      v-if="imgPath && imgPath.length > 0"
      id="img-marking"
      :src="imgPath"
      class="auto-size"
      draggable="false"
      @load="setSize"
          @mousedown="handleMouseDown"
    @mouseup="handleMouseUp"
    @mousemove="handleMouseMove"
    @mouseover="handleMouseOver"
    @mouseleave="handleMouseLeave"
    >
    <div
      v-show="showCross"
      :class="'cross cross-vertical' + (cropImage ? ' blue_cross': '')"
      :style="{
        height: `${crossHeight}px`,
        top: `${containerTop}px`,
        left: `${mouseX}px`,
      }"
    />
    <div
      v-show="showCross"
      :class="'cross cross-horizontal' + (cropImage ? ' blue_cross': '')"
      :style="{
        width: `${crossWidth}px`,
        top: `${mouseY}px`,
        left: `${containerLeft}px`,
      }"
    />
    <template v-if="imgLoaded">
      <div
        v-for="(item, i) in rectList"
        :key="i"
        :class="i === selectedRectIndex ? 'rect rect-red' : 'rect rect-blue'"
        :style="{
          top: `${item.top * containerHeight + containerTop}px`,
          left: `${item.left * containerWidth + containerLeft}px`,
          width: `${(item.right - item.left) * containerWidth}px`,
          height: `${(item.bottom - item.top) * containerHeight}px`,
        }"
        @click.stop="rectClick(i)"
        @keydown.stop="handleKeyDown"
      />
      <div
        v-show="showDrawingRect"
        class="rect rect-red"
        :style="{
          top: `${
            Math.min(drawingPosition.startY, drawingPosition.endY) +
            containerTop
          }px`,
          left: `${
            Math.min(drawingPosition.startX, drawingPosition.endX) +
            containerLeft
          }px`,
          width: `${Math.abs(drawingPosition.startX - drawingPosition.endX)}px`,
          height: `${Math.abs(
            drawingPosition.startY - drawingPosition.endY
          )}px`,
        }"
      />
    </template>
  </div>
</template>
<script>
export default {
  name: 'ImageMarker',
  props: {
    imgPath: {
      type: String,
      required: true
    },
    rectList: {
      type: Array,
      required: true
    },
    recoginzingRect: {
      type: Array,
      required: true
    },
    polygonList: {
      type: Array,
      default: () => [],
      required: false
    },
    originalRect: {
      type: Object,
      required: true
    },
    selectedRectIndex: {
      type: Number,
      required: true
    },
    adjustOffsetTop: {
      type: Number,
      required: false,
      default: 0
    },
    adjustOffsetLeft: {
      type: Number,
      required: false,
      default: 0
    },
    cropImage: {
      type: Boolean,
      required: false
    },
    minimumSize: {
      type: Array,
      default: () => [50, 50]
    }
  },
  data() {
    return {
      imgLoaded: false,
      showCross: false,
      crossHeight: 0,
      crossWidth: 0,
      containerLeft: 0,
      containerRight: 0,
      containerTop: 0,
      containerBottom: 0,
      containerWidth: 0,
      containerHeight: 0,
      containerPositionX: 0,
      containerPositionY: 0,
      mouseX: 0,
      mouseY: 0,
      naturalWidth: 0,
      naturalHeight: 0,
      mouseOffset: 2,
      lastMouseDown: [0, 0],
      drawingRect: false,
      drawingPosition: {}
    }
  },
  computed: {
    showDrawingRect() {
      if (!this.drawingRect) {
        return false
      }
      if (
        Math.abs(this.drawingPosition.startX - this.drawingPosition.endX) < 0 &&
          Math.abs(this.drawingPosition.startY - this.drawingPosition.endY) < 0
      ) {
        return false
      }
      return true
    }
  },
  watch: {
    recoginzingRect: function(newv, oldv) {
      this.redrawImage()
    }
  },
  mounted() {
    window.addEventListener('resize', this.setSize)
  },
  unmounted() {
    window.removeEventListener('resize', this.setSize)
  },
  updated() {
    if (!this.imgLoaded) {
      console.log('set size againt')
      this.setSize({})
    }
  },
  methods: {
    // 根据图片尺寸设置准星长度+尺寸自适应
    handleRateChange(e) {
      console.log('RateChange: ', e)
    },
    setSize(e) {
      setTimeout(() => {
        this.setInnerSize(e)
      }, 100)
    },
    setInnerSize(e) {
      console.log('Image load: ', e)
      window.scrollTo(0, 0)
      // console.log('setSize reload: ', this.$parent)
      // console.log('SetSize reload', this.$parent.$el.firstChild.getBoundingClientRect())
      // var padding_rect = { left: 0, top: 0, right: 0, bottom: 0 }
      // if (this.$parent && this.$parent.$el && this.$parent.$el.firstChild && this.$parent.$el.firstChild.className === 'el-dialog') {
      //  const { top, bottom, left, right } = this.$parent.$el.firstChild.getBoundingClientRect()
      //  padding_rect = { left: left, top: top, right: right, bottom: bottom }
      // } else {
      //  // const { top, bottom, left, right } = this.$parent.$el.firstChild.getBoundingClientRect()
      //  // padding_rect = { left: left, top: top, right: right, bottom: bottom }
      // }

      const container = document.getElementById('img-marking')
      if (!container) return
      // if (this.cropImage && container.src.indexOf('data:image/') === 0) return
      const pos = this.$el.getBoundingClientRect()
      // const elOffsetTop = this.$el.offsetTop
      // console.log('Position: ', container, container.getBoundingClientRect(), elOffsetTop, container.offsetTop, container.offsetParent.offsetTop)
      const { top, bottom, left, right } = container.getBoundingClientRect()
      console.log('Client Rect', container.naturalWidth, container.naturalHeight, container.offsetLeft)
      console.log(`BoundingClientRect: ${top}, ${bottom}, ${left}, ${right}`)
      this.naturalWidth = container.naturalWidth
      this.naturalHeight = container.naturalHeight
      this.crossHeight = bottom - top
      this.crossWidth = right - left
      this.containerTop = container.offsetTop
      this.containerBottom = this.naturalHeight + container.offsetTop
      this.containerLeft = container.offsetLeft
      this.containerRight = right + container.offsetLeft
      this.containerWidth = right - left
      this.containerHeight = bottom - top
      this.containerPositionX = pos.left
      this.containerPositionY = pos.top
      // console.log('Container Position', this.containerPositionX, this.containerPositionY, bottom, this.containerBottom)
      this.$emit('update:originalRect', { left: this.containerLeft, top: this.containerTop, width: this.naturalWidth, height: this.naturalHeight })
      this.imgLoaded = !(top == 0 && left == 0 && bottom == 0 && right == 0)
      /**
      this.$nextTick(() => {
        var canvas = document.createElement('canvas')
        canvas.width = this.containerWidth
        canvas.height = this.containerHeight
        var ctx = canvas.getContext('2d')
        ctx.drawImage(container, 0, 0, this.containerWidth, this.containerHeight)
        ctx.fillStyle = '#fff8'
        for (var rt of this.recoginzingRect) {
          console.log('Rect', rt)
          ctx.fillRect(rt.x, rt.y, rt.w, rt.h)
        }
        container.src = canvas.toDataURL('image/png')
        canvas.remove()
      })
      */
    },
    redrawImage() {
      const container = document.getElementById('img-marking')
      if (!container) return
      if (!this.recoginzingRect || this.recoginzingRect.length <= 0) {
        container.src = this.imgPath
        return
      }
      // if (container.src.indexOf('data:image/png;') === 0) return
      var canvas = document.createElement('canvas')
      canvas.width = this.containerWidth
      canvas.height = this.containerHeight
      var that = this
      var img = new Image()
      img.src = this.imgPath
      img.crossOrigin = 'Anonymous'
      img.onload = function() {
        var ctx = canvas.getContext('2d')
        ctx.drawImage(img, 0, 0, that.containerWidth, that.containerHeight)
        ctx.strokeStyle = '#555a'
        ctx.font = '12px 微软雅黑'
        ctx.textAlign = 'left'
        ctx.textBaseline = 'middle'
        for (var rt of that.recoginzingRect) {
          ctx.fillStyle = '#fff5'
          ctx.fillRect(rt.x * that.containerWidth, rt.y * that.containerHeight, rt.w * that.containerWidth, rt.h * that.containerHeight)
          ctx.beginPath()
          ctx.rect(rt.x * that.containerWidth, rt.y * that.containerHeight, rt.w * that.containerWidth, rt.h * that.containerHeight)
          ctx.stroke()
          if (rt.label !== '') {
            var label = rt.label + ' ' + rt.score.toFixed(2)
            ctx.fillStyle = 'blue'
            // if ((rt.y * that.containerHeight - 12) < 6) {
            ctx.fillText(label, rt.x * that.containerWidth, rt.y * that.containerHeight + 12)
            // } else {
            //  ctx.fillText(label, rt.x * that.containerWidth, rt.y * that.containerHeight - 12)
            // }
          }
        }
        container.src = canvas.toDataURL('image/png')
        canvas.remove()
      }
    },
    // 鼠标移动
    handleMouseMove(e) {
      this.showCross =
          e.offsetX < this.containerRight && e.offsetY < this.containerBottom

      // console.log('Move ', this.showCross, e.offsetX, this.containerRight, e.offsetY, this.containerBottom)

      if (!this.showCross) {
        return
      }

      const container = document.getElementById('img-marking')
      const rect = container.getBoundingClientRect();
      // console.log('Relative: ', e.offsetX, e.offsetY)
      var scrollX = e.pageX - e.offsetX
      var scrollY = e.offsetY - e.clientY + (e.screenY - e.pageY) + (window.screen.height - window.screen.availHeight) - ((e.screenY - e.pageY) - (window.screen.height - window.screen.availHeight)) / 2
      // console.log('Scroll: ', scrollX, scrollY, (e.screenY - e.pageY), (window.screen.height - window.screen.availHeight))
      this.mouseX = Math.max(e.pageX - this.containerPositionX - this.mouseOffset + this.containerLeft + this.adjustOffsetLeft, 0)
      this.mouseY = Math.max(e.pageY - this.containerPositionY - this.mouseOffset + this.containerTop + this.adjustOffsetTop, 0) + scrollY
      if (this.drawingRect) {
        this.drawingPosition = {
          ...this.drawingPosition,
          endX: this.mouseX - this.containerLeft,
          endY: this.mouseY - this.containerTop
        }
      }
    },
    // 鼠标移进
    handleMouseOver() {
      this.showCross = true
    },
    // 鼠标移出
    handleMouseLeave() {
      this.showCross = false
    },
    // 鼠标按下
    handleMouseDown(e) {
      console.log(e)
      this.drawingRect = true
      this.lastMouseDown = [this.mouseX, this.mouseY]

      this.drawingPosition = {
        startX: this.mouseX - this.containerLeft - this.mouseOffset,
        startY: this.mouseY - this.containerTop - this.mouseOffset,
        endX: this.mouseX - this.containerLeft - this.mouseOffset,
        endY: this.mouseY - this.containerTop - this.mouseOffset
      }
      console.log(this.rectList)
    },
    // 鼠标按起
    handleMouseUp(e) {
      this.drawingRect = false
      if (e.button === 2) return
      if (Math.abs(this.mouseX - this.lastMouseDown[0]) < this.minimumSize[0] || Math.abs(this.mouseY - this.lastMouseDown[1]) < this.minimumSize[1]) {
        return
      }
      if (this.cropImage) { // If in the crop image model, the rect's w/h should be great than 100.
        if (Math.abs(this.mouseX - this.lastMouseDown[0]) < 100 || Math.abs(this.mouseY - this.lastMouseDown[1]) < 100) {
          return
        }
      }
      const { startX, startY, endX, endY } = this.drawingPosition
      const list = this.cropImage ? [] : this.rectList
      list.push({
        top: Math.min(startY, endY) / this.containerHeight,
        bottom: Math.max(startY, endY) / this.containerHeight,
        left: Math.min(startX, endX) / this.containerWidth,
        right: Math.max(startX, endX) / this.containerWidth
      })
      if (this.cropImage) {
        const size = { containerWidth: this.containerWidth, containerHeight: this.containerHeight, naturalWidth: this.naturalWidth, naturalHeight: this.naturalHeight }
        this.$emit('onCropImagedRect', list, size)
      } else {
        this.$emit('update:rectList', list)
        this.$emit('update:selectedRectIndex', list.length - 1)
        this.$emit('onSelectedRectIndex', list.length - 1)
      }
    },
    // 点击方块
    rectClick(i) {
      this.$emit('update:selectedRectIndex', i)
      this.$emit('onSelectedRectIndex', i)
    },
    handleKeyDown(e) {
      console.log(e)
    }
  }
}
</script>
  <style scoped>
  .image_marker_container {
    width: 100%;
    height: 100%;
    padding: 0px;
    margin: 0px;
    position: relative;
    user-select: none;
    z-index: 999;
  }

  .image_marker_container img {
    z-index: -1;
  }

  .auto-size {
    max-width: 100%;
    max-height: 100%;
  }

  .cross {
    position: absolute;
    background: #f85757;
    z-index: 99;
  }

  .cross-vertical {
    width: 2px;
  }

  .cross-horizontal {
    height: 2px;
  }

  .blue_cross {
    background: #2417d8;
  }

  .rect {
    position: absolute;
    z-index: 50;
  }

  .rect-red {
    border: 1px solid rgb(125, 5, 5);
    background: rgba(125, 5, 5, 0.3);
  }

  .rect-blue {
    border: 1px solid rgb(3, 38, 165);
    background: rgba(3, 38, 165, 0.3);
  }
  </style>
