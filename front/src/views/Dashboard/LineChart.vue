<template>
  <div :class="className" :style="{height:height,width:width}" />
</template>

<script>
import echarts from 'echarts'
import * as macarons from 'echarts/theme/macarons'
import resize from './mixins/resize'
import * as datetime from '@/utils/datetime.js'

export default {
  mixins: [resize],
  props: {
    className: {
      type: String,
      default: 'chart'
    },
    width: {
      type: String,
      default: '100%'
    },
    height: {
      type: String,
      default: '290px'
    },
    autoResize: {
      type: Boolean,
      default: true
    },
    chartData: {
      type: Object,
      required: true
    }
  },
  data() {
    return {
      chart: null
    }
  },
  watch: {
    chartData: {
      deep: true,
      handler(val) {
        this.setOptions(val)
      }
    }
  },
  mounted() {
    this.$nextTick(() => {
      this.initChart()
    })
  },
  beforeDestroy() {
    if (!this.chart) {
      return
    }
    this.chart.dispose()
    this.chart = null
  },
  methods: {
    initChart() {
      this.chart = echarts.init(this.$el, 'macarons')
      this.setOptions(this.chartData)
    },
    setOptions({ data, title } = {}) {
      console.log(data)
      const xaxis = data.map(t => new Date(t.timestamp).strftime('%H:%M:%S'))
      const recvdata = data.map(t => (t.network_recv_total / 1024.0) / (t.diff_time))
      const senddata = data.map(t => (t.network_send_total / 1024.0) / (t.diff_time))
      const readdata = data.map(t => (t.disk_read_total / 1024.0) / (t.diff_time))
      const writedata = data.map(t => (t.disk_write_total / 1024.0) / (t.diff_time))
      this.chart.setOption({
        title: {
          text: this.$t('吞吐量 (KB/s)'),
          x: 'center'
        },
        xAxis: {
          data: xaxis,
          boundaryGap: false,
          axisTick: {
            show: false
          }
        },
        grid: {
          left: 10,
          right: 10,
          bottom: 20,
          top: 40,
          containLabel: true
        },
        tooltip: {
          trigger: 'point',
          axisPointer: {
            type: 'cross'
          },
          formatter: function(it) {
            console.log(it)
            return it.map(t => t.marker + t.seriesName + ': ' + (t.value / 1024).toFixed(2) + 'KB/秒').join('<br/>')
          },
          padding: [5, 10]
        },
        yAxis: {
          axisTick: {
            show: false
          }
        },
        legend: {
          data: [this.$t('网络接收'), this.$t('网络发送'), this.$t('磁盘读取'), this.$t('磁盘写入')],
          left: 'right',
          top: '5px'
        },
        series: [{
          name: this.$t('网络接收'), itemStyle: {
            normal: {
              color: '#FF005A',
              lineStyle: {
                color: '#FF005A',
                width: 2
              }
            }
          },
          smooth: true,
          type: 'line',
          data: recvdata,
          animationDuration: 2800,
          animationEasing: 'cubicInOut'
        }, {
          name: this.$t('网络发送'), itemStyle: {
            normal: {
              color: '#FFDD5A',
              lineStyle: {
                color: '#FFDD5A',
                width: 2
              }
            }
          },
          smooth: true,
          type: 'line',
          data: senddata,
          animationDuration: 2800,
          animationEasing: 'cubicInOut'
        }, {
          name: this.$t('磁盘读取'), itemStyle: {
            normal: {
              color: '#EE22FA',
              lineStyle: {
                color: '#EE22FA',
                width: 2
              }
            }
          },
          smooth: true,
          type: 'line',
          data: readdata,
          animationDuration: 2800,
          animationEasing: 'cubicInOut'
        }, {
          name: this.$t('磁盘写入'), itemStyle: {
            normal: {
              color: '#DDAADA',
              lineStyle: {
                color: '#DDAADA',
                width: 2
              }
            }
          },
          smooth: true,
          type: 'line',
          data: writedata,
          animationDuration: 2800,
          animationEasing: 'cubicInOut'
        }]
      })
    }
  }
}
</script>
